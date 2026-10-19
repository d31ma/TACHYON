use super::compiler::NativeBuildOptions;
use super::config::NativeApplication;
use super::host::{
    GeneratedHost, first_line, native_tool_failure, run_tool, stage_application, write,
    write_host_source, xml_escape,
};
use super::ios_destination::IosDestination;
use super::ios_device_build::IosDeviceBuild;
use super::ios_rust_library::IosRustLibrary;
use super::ios_window_controls::IosWindowControls;
use super::ios_window_settings::IosWindowSettings;
use super::macos::MacOsHostGenerator;
use super::native_reproducibility::NativeReproducibility;
use super::routes::NativeRouteIndex;
use super::rust::RustCompanionOutput;
use super::swift_compile_inputs::SwiftCompileInputs;
use super::system_version::SystemVersion;
use crate::Failure;
use std::path::{Path, PathBuf};
use tachyon_contracts::NativeTarget;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct IosHostGenerator;

impl IosHostGenerator {
    const FRAMEWORK_ARGUMENTS: [&str; 6] = [
        "-framework",
        "UIKit",
        "-framework",
        "SwiftUI",
        "-framework",
        "WebKit",
    ];

    pub(super) async fn generate(
        application: &NativeApplication,
        index: &NativeRouteIndex,
        companions: &[super::registry::NativeCompanionInput],
        web_bundle: &Path,
        stage: &Path,
        options: &NativeBuildOptions,
    ) -> Result<GeneratedHost, Failure> {
        let destination = IosDestination::of(options);
        let bundle = stage.join(format!("{}.app", application.executable_name));
        stage_application(index, web_bundle, stage, &bundle)?;

        let companion = super::macos::stage_swift_companion(
            companions,
            stage,
            application.outgoing.yon_origin(),
        )?;
        let rust_companion = super::rust::RustCompanion::stage_for_host(
            companions,
            stage,
            application,
            super::rust::Linkage::Static,
        )?;
        let call = super::macos::companion_call(companion.as_deref(), rust_companion.as_deref());
        let swift_path = stage.join("project").join("TachyonHost.swift");
        write_host_source(&swift_path, &swift_source(application, call, index))?;
        let icons = stage_icons(application, web_bundle, &bundle, destination.signed).await?;
        let platform = destination.plist_keys("");
        let entitlements =
            super::ios_entitlements::IosEntitlements::write(application, &stage.join("project"))?;
        write(
            &bundle.join("Info.plist"),
            info_plist(application, &icons, &platform).as_bytes(),
        )?;
        if !options.package {
            return Ok(GeneratedHost::not_packaged(PathBuf::from(
                "project/TachyonHost.swift",
            )));
        }

        let device = IosDeviceBuild::prepare(application, destination).await?;
        let executable = bundle.join(&application.executable_name);
        let deployment = application.minimum_system.floor(NativeTarget::Ios);
        let project = stage.join("project");
        let library = IosRustLibrary::compile(
            application,
            rust_companion.as_deref(),
            destination,
            &project,
        )
        .await?;
        let swift_version = compile_swift(
            &swift_path,
            (companion.as_deref(), library.as_ref()),
            destination,
            deployment,
            &executable,
        )
        .await?;
        super::ios_launch_screen::IosLaunchScreen::compile(
            &application.window,
            &project,
            &bundle,
            (destination.sdk, deployment),
        )
        .await?;
        let application_bundle = IosDeviceBuild::finish(
            device,
            application,
            web_bundle,
            stage,
            &bundle,
            entitlements.as_deref(),
        )
        .await?;
        Ok(GeneratedHost {
            application_bundle,
            toolchain_name: String::from("swift"),
            toolchain_version: swift_version,
        })
    }
}

async fn compile_swift(
    source: &Path,
    (companion, library): (Option<&Path>, Option<&RustCompanionOutput>),
    destination: IosDestination,
    deployment: SystemVersion,
    executable: &Path,
) -> Result<String, Failure> {
    if !cfg!(target_os = "macos") {
        return Err(native_tool_failure(
            1605,
            "The iOS host requires an Apple macOS build machine with Xcode.",
        ));
    }
    let version_output = run_tool(
        "/usr/bin/xcrun",
        &["--sdk", destination.sdk, "swiftc", "--version"],
    )
    .await?;
    let version = first_line(&version_output, "Swift unknown");
    let archive = library.map(|compiled| compiled.library.as_path());
    let inputs = SwiftCompileInputs::of(source, companion, archive, executable)?;
    let triple = destination.triple(deployment);
    let mut arguments = vec![
        "--sdk",
        destination.sdk,
        "swiftc",
        "-parse-as-library",
        "-O",
        "-target",
        &triple,
    ];
    arguments.extend(IosHostGenerator::FRAMEWORK_ARGUMENTS);
    arguments.extend(NativeReproducibility::SWIFT_ARGUMENTS);
    arguments.extend(inputs.arguments());
    let native_arguments = library
        .into_iter()
        .flat_map(|compiled| compiled.native_libraries.arguments.iter())
        .map(String::as_str);
    arguments.extend(native_arguments);
    NativeReproducibility::run_tool_in("/usr/bin/xcrun", &arguments, Some(&inputs.folder)).await?;
    Ok(version)
}

async fn stage_icons(
    application: &NativeApplication,
    web_bundle: &Path,
    bundle: &Path,
    signed: bool,
) -> Result<Vec<String>, Failure> {
    let Some(source) = application
        .largest_icon()
        .filter(|_| [super::apple_icons::AppleIcons::can_convert(), signed] == [true, false])
    else {
        return Ok(Vec::new());
    };
    let origin = web_bundle.join(source.trim_start_matches('/'));
    if !origin.is_file() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for (points, scale) in [(60u32, 2u32), (60, 3), (76, 2), (83, 2), (29, 2), (40, 2)] {
        let pixels = points * scale;
        let name = format!("AppIcon{points}x{points}@{scale}x.png");
        run_tool(
            "sips",
            &[
                "-z",
                &pixels.to_string(),
                &pixels.to_string(),
                &origin.to_string_lossy(),
                "--out",
                &bundle.join(&name).to_string_lossy(),
            ],
        )
        .await?;
        names.push(name);
    }
    Ok(names)
}

fn icon_block(icons: &[String]) -> String {
    if icons.is_empty() {
        return String::new();
    }
    let mut files = String::new();
    for name in icons {
        files.push_str("<string>");
        files.push_str(name);
        files.push_str("</string>");
    }
    format!(
        "\n<key>CFBundleIcons</key><dict><key>CFBundlePrimaryIcon</key><dict>\
         <key>CFBundleIconFiles</key><array>{files}</array>\
         </dict></dict>"
    )
}

pub(super) fn info_plist(
    application: &NativeApplication,
    icons: &[String],
    platform: &str,
) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleDevelopmentRegion</key><string>en</string>
<key>CFBundleDisplayName</key><string>{name}</string>
<key>CFBundleExecutable</key><string>{executable}</string>
<key>CFBundleIdentifier</key><string>{identifier}</string>{icon_block}
<key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
<key>CFBundleName</key><string>{name}</string>{location}
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>{version}</string>{url_types}
<key>CFBundleVersion</key><string>{build}</string>{platform}
<key>MinimumOSVersion</key><string>{deployment}</string>
<key>UIDeviceFamily</key><array><integer>1</integer><integer>2</integer></array>
{launch_screen}{window}
</dict></plist>
"#,
        name = xml_escape(&application.name),
        executable = application.executable_name,
        identifier = application.application_id,
        version = application.version,
        build = application.build,
        deployment = application.minimum_system.floor(NativeTarget::Ios),
        icon_block = icon_block(icons),
        location = application.permissions.location_plist_entries(
            &super::browser_api_permissions::BrowserApiPermissions::IOS_LOCATION_KEYS
        ),
        url_types = application
            .links
            .url_types_plist(&application.application_id),
        launch_screen = super::ios_launch_screen::IosLaunchScreen::plist_entry(&application.window),
        window = IosWindowSettings::plist_entries(&application.window),
    )
}

pub(super) fn swift_source(
    application: &NativeApplication,
    companion_call: &str,
    index: &NativeRouteIndex,
) -> String {
    let rust_shim =
        ["", MacOsHostGenerator::RUST_SHIM][usize::from(companion_call.contains("tacRustInvoke"))];
    let framed = IosHostGenerator::SWIFT_HOST
        .replace(
            "__TACHYON_JSON_REQUEST__",
            MacOsHostGenerator::APPLE_JSON_REQUEST,
        )
        .replace(
            "__NATIVE_BROWSER_APIS__",
            super::apple_browser_apis::AppleBrowserApis::SWIFT,
        )
        .replace(
            "__NATIVE_SHIM__",
            &format!(
                "{}{}",
                super::host::native_shim(
                    &IosWindowControls::page_window(&application.window),
                    tachyon_contracts::NativeTarget::Ios,
                ),
                application.window.title_bar.mobile_page_script()
            ),
        )
        .replace(
            "__IOS_WINDOW_CONTROLS__",
            &format!(
                "{}{}",
                IosWindowControls::bridge_cases(&application.window),
                IosWindowSettings::bridge_case(&application.window)
            ),
        )
        .replace(
            "__IOS_WINDOW_SETTINGS__",
            &IosWindowSettings::swift(&application.window),
        )
        .replace(
            "__NATIVE_CAPABILITY_REGISTER__",
            MacOsHostGenerator::capability_registration(companion_call),
        )
        .replace("__TACHYON_COMPANION_CALL__", companion_call)
        .replace(
            "__TACHYON_COMPANION_EMIT__",
            super::macos::companion_emit(companion_call),
        )
        .replace("__TACHYON_RUST_SHIM__", rust_shim)
        .replace(
            "__NATIVE_APPLICATION_STATE__",
            include_str!("AppleApplicationState.swift"),
        )
        .replace(
            "__NATIVE_MENUS__",
            &[
                include_str!("menus/IosMenuRoute.swift"),
                include_str!("menus/IosMenus.swift"),
            ]
            .join("\n"),
        );
    let menu_routes = super::mobile_menu_source::MobileMenuSource::swift_routes(&application.menus);
    MacOsHostGenerator::apple_authored_values(
        &framed,
        application,
        index,
        &[("__NATIVE_MENU_ROUTES__", &menu_routes)],
    )
}

#[cfg(all(test, target_os = "macos"))]
pub(super) async fn link_swift(
    source: &Path,
    library: &RustCompanionOutput,
    destination: IosDestination,
    executable: &Path,
) -> Result<String, Failure> {
    let deployment = super::target_floor::TargetFloor::of(NativeTarget::Ios).base;
    compile_swift(
        source,
        (None, Some(library)),
        destination,
        deployment,
        executable,
    )
    .await
}

#[cfg(test)]
pub(super) const fn host_source() -> &'static str {
    IosHostGenerator::SWIFT_HOST
}

impl IosHostGenerator {
    const SWIFT_HOST: &str = r##"import Foundation
import UIKit
import WebKit

__TACHYON_RUST_SHIM__
__TACHYON_JSON_REQUEST__
__NATIVE_BROWSER_APIS__

private let tachyonEntryDocument = "__ENTRY_DOCUMENT__"
private let tachyonRouteDocuments: [String: String] = __ROUTE_DOCUMENTS__
__NATIVE_DEEP_LINKS__

private func tachyonRoute(_ path: String) -> String? {
    guard path.utf8.count <= 2048, !path.contains("\\"), !path.split(separator: "/").contains("..") else { return nil }
    if tachyonRouteDocuments[path] != nil { return path }
    if let document = tachyonRouteDocuments.first(where: { "/" + $0.value == path }) { return document.key }
    let parts = path.split(separator: "/")
    let patterns = tachyonRouteDocuments.keys.sorted {
        let a = $0.split(separator: "/").filter { !$0.hasPrefix("_") }.count
        let b = $1.split(separator: "/").filter { !$0.hasPrefix("_") }.count
        return a == b ? $0 < $1 : a > b
    }
    for route in patterns {
        let expected = route.split(separator: "/")
        if expected.count == parts.count && zip(expected, parts).allSatisfy({ $0.0.hasPrefix("_") || $0.0 == $0.1 }) { return route }
    }
    return nil
}

private func tachyonDocument(_ path: String) -> String? {
    if let route = tachyonRoute(path) { return tachyonRouteDocuments[route] }
    let parts = path.split(separator: "/")
    guard parts.count > 1 else { return nil }
    for count in stride(from: parts.count - 1, through: 1, by: -1) {
        let prefix = "/" + parts.prefix(count).joined(separator: "/")
        if let route = tachyonRoute(prefix), route.contains("/_"), let document = tachyonRouteDocuments[route] {
            let directory = (document as NSString).deletingLastPathComponent
            return directory + "/" + parts.dropFirst(count).joined(separator: "/")
        }
    }
    return nil
}

@MainActor
private final class Telemetry {
    static let shared = Telemetry()
    var route = "__ENTRY_ROUTE__"

    func record(_ event: String) {
        let allowed = String(event.prefix(128))
        let directory = FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Logs/Tachyon", isDirectory: true)
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let url = directory.appendingPathComponent("__BUNDLE_ID__.jsonl")
        let entry = "{\"event\":\"" + allowed + "\",\"route\":\"" + String(route.prefix(256)) + "\"}\n"
        guard let data = entry.data(using: .utf8) else { return }
        if !FileManager.default.fileExists(atPath: url.path) {
            FileManager.default.createFile(atPath: url.path, contents: data)
        } else if let handle = try? FileHandle(forWritingTo: url) {
            try? handle.seekToEnd()
            try? handle.write(contentsOf: data)
            try? handle.close()
        }
    }
}

private final class TachyonAssetSchemeHandler: NSObject, WKURLSchemeHandler {
    static let shared = TachyonAssetSchemeHandler()

    func webView(_ webView: WKWebView, start task: WKURLSchemeTask) {
        guard let url = task.request.url,
              url.scheme == "tachyon-app",
              url.host == "bundle", url.port == nil, url.user == nil, url.password == nil,
              let resources = Bundle.main.resourceURL
        else {
            task.didFailWithError(NSError(domain: "TachyonAsset", code: 1))
            return
        }
        let root = resources.appendingPathComponent("WebBundle").standardizedFileURL
        var path = url.path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        if let document = tachyonDocument(url.path) { path = document }
        var file = root.appendingPathComponent(path).standardizedFileURL
        var isDirectory: ObjCBool = false
        if !FileManager.default.fileExists(atPath: file.path, isDirectory: &isDirectory)
            || isDirectory.boolValue {
            file = root.appendingPathComponent(path)
                .appendingPathComponent("index.html").standardizedFileURL
        }
        file = file.resolvingSymlinksInPath()
        guard !path.split(separator: "/").contains(".."),
              file.path.hasPrefix(root.resolvingSymlinksInPath().path + "/"),
              let attributes = try? FileManager.default.attributesOfItem(atPath: file.path),
              let size = attributes[.size] as? NSNumber, size.uint64Value <= 16 * 1024 * 1024,
              let data = try? Data(contentsOf: file)
        else {
            task.didFailWithError(NSError(domain: "TachyonAsset", code: 2))
            return
        }
        let response = HTTPURLResponse(
            url: url,
            statusCode: 200,
            httpVersion: "HTTP/1.1",
            headerFields: [
                "Content-Type": TachyonAssetSchemeHandler.mimeType(file.pathExtension),
                "Content-Length": String(data.count),
                "Content-Security-Policy": "__CONTENT_SECURITY_POLICY__",
            ]
        )
        guard let response else {
            task.didFailWithError(NSError(domain: "TachyonAsset", code: 3))
            return
        }
        task.didReceive(response)
        task.didReceive(data)
        task.didFinish()
    }

    func webView(_ webView: WKWebView, stop task: WKURLSchemeTask) {}

    private static func mimeType(_ extensionName: String) -> String {
        switch extensionName.lowercased() {
        case "html": return "text/html"
        case "css": return "text/css"
        case "js", "mjs": return "text/javascript"
        case "json", "map": return "application/json"
        case "svg": return "image/svg+xml"
        case "png": return "image/png"
        case "jpg", "jpeg": return "image/jpeg"
        case "webp": return "image/webp"
        case "woff2": return "font/woff2"
        case "wasm": return "application/wasm"
        default: return "application/octet-stream"
        }
    }
}


@MainActor
private final class NativeBridge: NSObject, WKScriptMessageHandlerWithReply {
    static let shared = NativeBridge()
    weak var webView: WKWebView?

    func userContentController(
        _ controller: WKUserContentController,
        didReceive message: WKScriptMessage,
        replyHandler: @escaping (Any?, String?) -> Void
    ) {
        guard message.frameInfo.isMainFrame,
              message.frameInfo.request.url?.scheme == "tachyon-app",
              message.frameInfo.request.url?.host == "bundle",
              message.frameInfo.request.url?.port == nil, message.frameInfo.request.url?.user == nil,
              message.frameInfo.request.url?.password == nil,
              let body = message.body as? [String: Any],
              let capability = body["capability"] as? String
        else {
            replyHandler(nil, "malformed host call")
            return
        }
        let payload = body["payload"] as? String ?? "{}"
        guard capability.utf8.count <= 64,
              let request = tachyonParseJSONRequest(payload) else {
            replyHandler(nil, "invalid or oversized host call"); return
        }
        if capability == "companion.invoke" {
            guard let operation = request["op"] as? String,
                  ["init", "get", "set", "call", "event", "dispose"].contains(operation),
                  let path = message.frameInfo.request.url?.path,
                  let route = tachyonRoute(path), request["route"] as? String == route else {
                replyHandler(nil, "companion route does not belong to this page"); return
            }
        }
        if capability.hasPrefix("browserApi.") {
            AppleBrowserApis.shared.call(capability, request, webView) { replyHandler($0, nil) }
            return
        }
        guard let canonicalPayload = tachyonCanonicalJSONRequest(request) else {
            replyHandler(nil, "invalid or oversized host call"); return
        }
        replyHandler(handle(capability, canonicalPayload), nil)
    }

    fileprivate func handle(_ capability: String, _ payload: String) -> String {
        let data = payload.data(using: .utf8) ?? Data()
        let parsed = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] ?? [:]
        switch capability {
__IOS_WINDOW_CONTROLS__
        case "companion.invoke":
            return __TACHYON_COMPANION_CALL__
        case "route.open":
            let route = String((parsed["route"] as? String ?? "/").prefix(256))
            Telemetry.shared.route = route
            webView?.evaluateJavaScript("location.assign(\(NativeBridge.jsonString(route)))")
            Telemetry.shared.record("route.opened")
        default:
            return "{\"ok\":false,\"error\":\"ios host does not implement capability '\(NativeBridge.safeName(capability))'\"}"
        }
        return "{\"ok\":true,\"value\":{}}"
    }

    private static func jsonString(_ value: String) -> String {
        let data = (try? JSONSerialization.data(withJSONObject: [value])) ?? Data("[\"/\"]".utf8)
        let text = String(data: data, encoding: .utf8) ?? "[\"/\"]"
        return String(text.dropFirst().dropLast())
    }

    private static func safeName(_ value: String) -> String {
        let allowed = value.prefix(64).filter { character in
            character.isLetter || character.isNumber || character == "." || character == "_" || character == "-"
        }
        return allowed.isEmpty ? "unnamed" : String(allowed)
    }
}

private let TACHYON_NATIVE_SHIM = #"""
__NATIVE_SHIM__
"""#

__NATIVE_APPLICATION_STATE__

__IOS_WINDOW_SETTINGS__

__NATIVE_MENUS__

private func tachyonRelayPublish(_ payload: String) {
    Task { @MainActor in
        NativeBridge.shared.webView?.evaluateJavaScript(
            "globalThis.__tachyonCompanionPublish(" + payload + ")")
    }
}

private final class RootViewController: UIViewController, WKNavigationDelegate {
    private var webView: WKWebView?
    var launchTarget: String?

    override var prefersStatusBarHidden: Bool { IosWindowSettings.fullscreen }
    override var prefersHomeIndicatorAutoHidden: Bool { IosWindowSettings.fullscreen }
    override var preferredStatusBarStyle: UIStatusBarStyle { IosWindowSettings.statusBarStyle }

    override func viewSafeAreaInsetsDidChange() {
        super.viewSafeAreaInsetsDidChange()
        IosWindowSettings.publish(view, webView)
    }

    func open(_ target: String) {
        guard let url = URL(string: target) else { return }
        Telemetry.shared.record("link.opened")
        guard let view = webView else { launchTarget = target; return }
        view.load(URLRequest(url: url))
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        Telemetry.shared.record("controller.created")
        let configuration = WKWebViewConfiguration()
        configuration.setURLSchemeHandler(TachyonAssetSchemeHandler.shared, forURLScheme: "tachyon-app")
        configuration.websiteDataStore = .default()
        configuration.userContentController.addUserScript(
            WKUserScript(source: TACHYON_NATIVE_SHIM, injectionTime: .atDocumentStart, forMainFrameOnly: true)
        )
        configuration.userContentController.addScriptMessageHandler(
            NativeBridge.shared, contentWorld: .page, name: "tachyon"
        )
        let view = WKWebView(frame: .zero, configuration: configuration)
        view.navigationDelegate = self
        view.accessibilityIdentifier = "tachyon-application"
        view.translatesAutoresizingMaskIntoConstraints = false
        IosWindowSettings.prepare(self.view, view)
        self.view.addSubview(view)
        NSLayoutConstraint.activate([
            view.topAnchor.constraint(equalTo: IosWindowSettings.topAnchor(self.view)),
            view.bottomAnchor.constraint(equalTo: self.view.bottomAnchor),
            view.leadingAnchor.constraint(equalTo: self.view.leadingAnchor),
            view.trailingAnchor.constraint(equalTo: self.view.trailingAnchor),
        ])
        webView = view
        NativeBridge.shared.webView = view
        __TACHYON_COMPANION_EMIT__
        let first = launchTarget.flatMap { URL(string: $0) } ?? URL(string: "tachyon-app://bundle/" + tachyonEntryDocument)!
        launchTarget = nil
        AppleBrowserApis.shared.prepare(view) {
            view.load(URLRequest(url: first))
        }
        Telemetry.shared.record("bridge.ready")
    }

    override var keyCommands: [UIKeyCommand]? {
        IosMenus.shortcuts()
    }

    @objc func openMenuRoute(_ command: UICommand) {
        guard let url = IosMenus.destination(command) else { return }
        Telemetry.shared.record("menu.opened")
        webView?.load(URLRequest(url: url))
    }

    func webView(_ webView: WKWebView, decidePolicyFor action: WKNavigationAction, decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
        guard let url = action.request.url else { decisionHandler(.cancel); return }
        if url.scheme == "tachyon-app" && url.host == "bundle" && url.port == nil && url.user == nil && url.password == nil { decisionHandler(.allow); return }
        decisionHandler(.cancel)
        let scheme = url.scheme?.lowercased() ?? ""
        let web = ["https", "http"].contains(scheme) && url.host?.isEmpty == false && url.user == nil && url.password == nil
        let mail = scheme == "mailto" && url.absoluteString.count > "mailto:".count
        if action.targetFrame?.isMainFrame != false && (web || mail) {
            UIApplication.shared.open(url)
        }
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        Telemetry.shared.record("controller.mounted")
        Telemetry.shared.record("controller.active")
        AppleApplicationState.shared.post()
    }

    func webView(_ webView: WKWebView, didStartProvisionalNavigation navigation: WKNavigation!) {
        tachyonDisposeCompanions()
    }

    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
        Telemetry.shared.record("route.failed")
    }
}

@UIApplicationMain
private final class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions options: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        AppleBrowserApis.shared.granted = __NATIVE_BROWSER_API_GRANTS__
        __NATIVE_CAPABILITY_REGISTER__
        let window = UIWindow(frame: UIScreen.main.bounds)
        let root = RootViewController()
        if let link = options?[.url] as? URL, let target = TacDeepLink.target(link.absoluteString) {
            root.open(target)
        }
        window.rootViewController = root
        window.makeKeyAndVisible()
        self.window = window
        AppleApplicationState.shared.watch([
            UIApplication.didEnterBackgroundNotification: "background",
            UIApplication.willEnterForegroundNotification: "foreground",
        ])
        return true
    }

    func application(_ application: UIApplication, open url: URL, options: [UIApplication.OpenURLOptionsKey: Any] = [:]) -> Bool {
        guard let target = TacDeepLink.target(url.absoluteString) else { return false }
        (window?.rootViewController as? RootViewController)?.open(target)
        return true
    }

    func application(
        _ application: UIApplication, continue activity: NSUserActivity,
        restorationHandler: @escaping ([UIUserActivityRestoring]?) -> Void
    ) -> Bool {
        guard activity.activityType == NSUserActivityTypeBrowsingWeb, let url = activity.webpageURL,
              let target = TacDeepLink.target(url.absoluteString) else { return false }
        (window?.rootViewController as? RootViewController)?.open(target)
        return true
    }

    override func buildMenu(with builder: UIMenuBuilder) {
        super.buildMenu(with: builder)
        IosMenus.build(builder)
    }

    func applicationWillTerminate(_ application: UIApplication) {
        tachyonDisposeCompanions()
        Telemetry.shared.record("controller.destroyed")
    }
}

private func tachyonDisposeCompanions() {
    let payload = "{\"op\":\"host-dispose\"}"
    _ = __TACHYON_COMPANION_CALL__
}
"##;
}

#[cfg(test)]
mod tests {
    use super::{SystemVersion, info_plist, swift_source};
    use crate::native::config::NativeApplication;
    use crate::native::ios_destination::IosDestination;
    use tachyon_contracts::NativeTarget;

    fn application() -> NativeApplication {
        NativeApplication {
            name: String::from("Native Catalog"),
            executable_name: String::from("NativeCatalog"),
            application_id: String::from("dev.tachyon.native-catalog"),
            version: String::from("1.0.0"),
            build: 1,
            entry_route: String::from("/"),
            icons: Vec::new(),
            window: crate::native::config::WindowConfiguration::default(),
            native_capabilities: Vec::new(),
            menus: crate::native::native_menu_config::NativeMenuConfig::default(),
            rust: crate::native::native_rust_project::NativeRustProject::default(),
            outgoing: crate::native::native_outgoing_hosts::NativeOutgoingHosts::default(),
            permissions: crate::native::browser_api_permissions::BrowserApiPermissions::default(),
            minimum_system: crate::native::minimum_system::MinimumSystem::default(),
            description: String::new(),
            tray: None,
            links: crate::native::native_links::NativeLinks::default(),
        }
    }

    fn index() -> crate::native::routes::NativeRouteIndex {
        crate::native::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: Vec::new(),
        }
    }

    #[test]
    fn an_entry_route_that_looks_like_a_placeholder_stays_text() {
        let mut routed = index();
        routed.entry_route = String::from("/__BUNDLE_ID__/__TACHYON_COMPANION_CALL__");
        let source = swift_source(&application(), "tacNativeInvoke(payload)", &routed);
        assert!(source.contains(r#"var route = "/__BUNDLE_ID__/__TACHYON_COMPANION_CALL__""#));
    }

    #[test]
    fn the_host_is_a_full_screen_view_over_the_bundle() {
        let source = swift_source(&application(), "tacNativeInvoke(payload)", &index());
        assert!(source.contains("import UIKit"));
        assert!(!source.contains("import AppKit"));
        assert!(source.contains("controller.created"));
        assert!(source.contains("TachyonAssetSchemeHandler"));
        assert!(source.contains("tachyon-app://bundle/"));
        assert!(source.contains("WKScriptMessageHandlerWithReply"));
        assert!(source.contains("didStartProvisionalNavigation"));
        assert!(source.contains("tachyonDisposeCompanions()"));
        assert!(
            source.contains(
                r#"["init", "get", "set", "call", "event", "dispose"].contains(operation)"#
            )
        );
        assert!(!source.contains("UIViewRepresentable"));
    }

    #[test]
    fn plist_declares_a_launchable_simulator_bundle() {
        let plist = info_plist(
            &application(),
            &[String::from("AppIcon60x60@2x.png")],
            &IosDestination::SIMULATOR.plist_keys(""),
        );
        assert!(plist.contains("dev.tachyon.native-catalog"));
        assert!(plist.contains("<string>iPhoneSimulator</string>"));
        assert!(plist.contains("<key>UILaunchScreen</key>"));
        assert!(plist.contains("<key>MinimumOSVersion</key><string>17.0</string>"));
    }

    #[test]
    fn a_raised_floor_reaches_the_plist() {
        let mut raised = application();
        raised.minimum_system = raised
            .minimum_system
            .with_floor(NativeTarget::Ios, SystemVersion::new(18, 2));
        let plist = info_plist(&raised, &[], &IosDestination::SIMULATOR.plist_keys(""));
        assert!(plist.contains("<key>MinimumOSVersion</key><string>18.2</string>"));
    }

    #[test]
    fn plist_carries_the_version_and_the_build() {
        let mut application = application();
        application.version = String::from("2.1.0");
        application.build = 7;
        let plist = info_plist(&application, &[], &IosDestination::SIMULATOR.plist_keys(""));
        assert!(plist.contains("<key>CFBundleShortVersionString</key><string>2.1.0</string>"));
        assert!(plist.contains("<key>CFBundleVersion</key><string>7</string>"));
    }

    #[test]
    fn a_device_plist_runs_on_iphoneos() {
        let keys = IosDestination::DEVICE
            .plist_keys("\n<key>DTSDKName</key><string>iphoneos18.2</string>");
        let plist = info_plist(&application(), &[], &keys);
        assert!(plist.contains(
            "<key>CFBundleSupportedPlatforms</key><array><string>iPhoneOS</string></array>"
        ));
        assert!(plist.contains("<key>LSRequiresIPhoneOS</key><true/>"));
        assert!(plist.contains("<key>DTSDKName</key><string>iphoneos18.2</string>"));
        assert!(!plist.contains("iPhoneSimulator"));
        assert!(plist.contains("<key>MinimumOSVersion</key>"));
    }

    #[test]
    fn plist_carries_the_manifest_reason_for_location() {
        assert!(
            !info_plist(
                &application(),
                &[],
                &IosDestination::SIMULATOR.plist_keys("")
            )
            .contains("NSLocation")
        );
        let mut located = application();
        located.permissions =
            crate::native::browser_api_permissions::BrowserApiPermissions::declared(
                &[String::from("geolocation"), String::from("notifications")],
                &std::collections::BTreeMap::from([
                    (
                        String::from("geolocation"),
                        String::from("Shows stores & more"),
                    ),
                    (String::from("notifications"), String::from("Order updates")),
                ]),
            );
        let plist = info_plist(&located, &[], &IosDestination::SIMULATOR.plist_keys(""));
        assert!(plist.contains(
            "<key>NSLocationWhenInUseUsageDescription</key><string>Shows stores &amp; more</string>"
        ));
    }

    #[test]
    fn a_linked_swift_service_reaches_the_window_controls_and_the_window_settings_are_built_in() {
        let linked = swift_source(&application(), "tacNativeInvoke(payload)", &index());
        assert!(linked.contains("NativeCapabilities.call = { capability, payload in"));
        assert!(linked.contains("private enum IosWindowSettings"));
        assert!(linked.contains("IosWindowSettings.prepare(self.view, view)"));
        assert!(linked.contains("override var prefersStatusBarHidden: Bool"));
        assert!(!linked.contains("__IOS_WINDOW_SETTINGS__"));
        assert!(!linked.contains("__NATIVE_CAPABILITY_REGISTER__"));
        assert!(!linked.contains("__tachyonTitleBar ??="));
        let alone = swift_source(&application(), "\"{}\"", &index());
        assert!(!alone.contains("NativeCapabilities.call ="));
        let mut integrated = application();
        integrated.window.title_bar = crate::native::native_title_bar::NativeTitleBar::Integrated;
        let source = swift_source(&integrated, "tacNativeInvoke(payload)", &index());
        assert!(source.contains("globalThis.__tachyonTitleBar ??= new (class TacMobileTitleBar"));
        assert!(source.contains("case \"window.titleBarInsets\":"));
    }

    #[test]
    fn the_plist_follows_the_manifest_orientation_and_background_color() {
        let mut turned = application();
        turned.window.orientation = crate::native::native_orientation::NativeOrientation::Landscape;
        turned.window.background_color =
            Some(crate::native::native_theme_color::NativeThemeColor {
                red: 1,
                green: 2,
                blue: 3,
            });
        let plist = info_plist(&turned, &[], &IosDestination::SIMULATOR.plist_keys(""));
        assert!(plist.contains("<string>UIInterfaceOrientationLandscapeLeft</string>"));
        assert!(!plist.contains("UIInterfaceOrientationPortrait"));
        assert!(plist.contains("<key>UIColorName</key><string>TachyonLaunchBackground</string>"));
        let default = info_plist(
            &application(),
            &[],
            &IosDestination::SIMULATOR.plist_keys(""),
        );
        assert!(default.contains("<string>UIInterfaceOrientationPortraitUpsideDown</string>"));
    }

    #[test]
    fn the_host_sends_the_application_state_when_the_app_enters_the_background_and_returns() {
        let source = swift_source(&application(), "tacNativeInvoke(payload)", &index());
        assert!(source.contains("UIApplication.didEnterBackgroundNotification: \"background\""));
        assert!(source.contains("UIApplication.willEnterForegroundNotification: \"foreground\""));
        assert!(source.contains("beginBackgroundTask"));
        assert!(source.contains("globalThis.__tachyonApplicationState?.('"));
        assert!(source.contains("AppleApplicationState.shared.post()"));
        assert!(!source.contains("__NATIVE_APPLICATION_STATE__"));
    }
}
