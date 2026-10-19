use super::config::NativeApplication;
use super::host::{
    GeneratedHost, native_io, native_tool_failure, quoted_string_escape, run_tool,
    stage_application, write, write_host_source, xml_escape,
};
use super::routes::NativeRouteIndex;
use crate::Failure;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MacOsHostGenerator;

impl MacOsHostGenerator {
    const SWIFT_COMPANION_PRELUDE: &str = concat!(
        include_str!("prelude.swift"),
        include_str!("TacServicePages.swift"),
        include_str!("prelude_apple.swift"),
        include_str!("TacAwaited.swift"),
        include_str!("TacOwnerThread.swift"),
        include_str!("capabilities/FilePickerResult.swift"),
        include_str!("capabilities/FilePickerStatus.swift"),
        include_str!("capabilities/FilePickerOptions.swift"),
        include_str!("capabilities/FilePicker.swift"),
        include_str!("capabilities/NativeCapabilities.swift"),
        include_str!("capabilities/NativeCapabilityScope.swift"),
        include_str!("capabilities/WindowControls.swift"),
        include_str!("capabilities/NativeCapabilityFailure.swift")
    );
    pub(super) const APPLE_JSON_REQUEST: &str = include_str!("apple_json.swift");
    pub(super) async fn generate(
        application: &NativeApplication,
        index: &NativeRouteIndex,
        companions: &[super::registry::NativeCompanionInput],
        web_bundle: &Path,
        stage: &Path,
        options: &super::compiler::NativeBuildOptions,
    ) -> Result<GeneratedHost, Failure> {
        let bundle = stage.join(format!("{}.app", application.executable_name));
        let contents = bundle.join("Contents");
        let resources = contents.join("Resources");
        stage_application(index, web_bundle, stage, &resources)?;

        let companion =
            stage_swift_companion(companions, stage, application.outgoing.yon_origin())?;
        let rust_companion = super::rust::RustCompanion::stage_for_host(
            companions,
            stage,
            application,
            super::rust::Linkage::Static,
        )?;
        let swift_path = stage.join("project").join("TachyonHost.swift");
        let call = companion_call(companion.as_deref(), rust_companion.as_deref());
        write_host_source(&swift_path, &swift_source(application, call, index))?;
        let icon = stage_icon(application, web_bundle, &resources).await?;
        write(
            &contents.join("Info.plist"),
            info_plist(application, icon.as_deref()).as_bytes(),
        )?;
        if !options.package {
            return Ok(GeneratedHost::not_packaged(PathBuf::from(
                "project/TachyonHost.swift",
            )));
        }

        let executable = executable_path(application, &contents)?;
        let swift_version = super::mac_os_executable::MacOsExecutable {
            application,
            stage,
            source: &swift_path,
            companion: companion.as_deref(),
            rust_companion: rust_companion.as_deref(),
        }
        .build(options.macos_architecture, &executable)
        .await?;
        sign_bundle(&bundle).await?;
        Ok(GeneratedHost {
            application_bundle: PathBuf::from(format!("{}.app", application.executable_name)),
            toolchain_name: String::from("swift"),
            toolchain_version: swift_version,
        })
    }
}

fn executable_path(application: &NativeApplication, contents: &Path) -> Result<PathBuf, Failure> {
    let executable_directory = contents.join("MacOS");
    native_io(
        std::fs::create_dir_all(&executable_directory),
        &executable_directory,
    )?;
    Ok(executable_directory.join(&application.executable_name))
}

pub(super) fn stage_swift_companion(
    companions: &[super::registry::NativeCompanionInput],
    stage: &Path,
    yon_origin: Option<&str>,
) -> Result<Option<PathBuf>, Failure> {
    let Some(authored) =
        super::registry::source(companions, crate::project::NativeCompanion::Swift)?
    else {
        return Ok(None);
    };
    let staged = stage.join("project").join("TachyonCompanion.swift");
    let prelude = MacOsHostGenerator::SWIFT_COMPANION_PRELUDE;
    let authored = super::swift_yon_http::SwiftYonHttp::linked(&authored, yon_origin);
    let built_in =
        crate::route_types::WebEventSources::native_application_state("swift", &authored);
    write(
        &staged,
        format!("{authored}\n{built_in}{prelude}").as_bytes(),
    )?;
    Ok(Some(staged))
}

#[cfg(test)]
pub(super) const fn companion_prelude() -> &'static str {
    MacOsHostGenerator::SWIFT_COMPANION_PRELUDE
}

pub(super) fn companion_call(swift: Option<&Path>, rust: Option<&Path>) -> &'static str {
    if swift.is_some() && rust.is_some() {
        "{ () -> String in let request = ((try? JSONSerialization.jsonObject(with: Data(payload.utf8))) as? [String: Any]) ?? [:]; if request[\"op\"] as? String == \"host-dispose\" { _ = tacNativeInvoke(payload); return tacRustInvoke(payload) }; return tacServiceKnown(request[\"service\"] as? String ?? \"\") ? tacNativeInvoke(payload) : tacRustInvoke(payload) }()"
    } else if swift.is_some() {
        "tacNativeInvoke(payload)"
    } else if rust.is_some() {
        "tacRustInvoke(payload)"
    } else {
        r#"((try? JSONSerialization.jsonObject(with: Data(payload.utf8))) as? [String: Any])?["op"] as? String == "init" ? "{\"value\":{\"fields\":[],\"methods\":[]}}" : "{\"error\":\"This application has no native companion.\"}""#
    }
}

pub(super) fn companion_emit(companion_call: &str) -> &'static str {
    if companion_call.contains("tacRustInvoke") && companion_call.contains("tacNativeInvoke") {
        "TacBridge.emit = tachyonRelayPublish; tacRustNativeSetEmit(tacRustEmit); tacRustNativeSetCapabilityAdapter(tacRustCapabilityCall, tacRustCapabilityFree)"
    } else if companion_call.contains("tacRustInvoke") {
        "tacRustNativeSetEmit(tacRustEmit); tacRustNativeSetCapabilityAdapter(tacRustCapabilityCall, tacRustCapabilityFree)"
    } else if companion_call.contains("tacNativeInvoke") {
        "TacBridge.emit = tachyonRelayPublish"
    } else {
        "// No companion, so nothing publishes."
    }
}

pub(super) fn unicode_path<'a>(path: &'a Path, message: &str) -> Result<&'a str, Failure> {
    path.to_str()
        .ok_or_else(|| native_tool_failure(1605, message))
}

pub(super) async fn sign_bundle(bundle: &Path) -> Result<(), Failure> {
    let bundle = unicode_path(bundle, "Application path is not valid Unicode.")?;
    run_tool(
        "/usr/bin/codesign",
        &["--force", "--sign", "-", "--timestamp=none", bundle],
    )
    .await
    .map(|_| ())
}

async fn stage_icon(
    application: &NativeApplication,
    web_bundle: &Path,
    resources: &Path,
) -> Result<Option<String>, Failure> {
    let Some(source) = application
        .largest_icon()
        .filter(|_| super::apple_icons::AppleIcons::can_convert())
    else {
        return Ok(None);
    };
    let origin = web_bundle.join(source.trim_start_matches('/'));
    if !origin.is_file() {
        return Ok(None);
    }
    let iconset = resources.join("AppIcon.iconset");
    super::host::native_io(std::fs::create_dir_all(&iconset), &iconset)?;
    write_iconset(&origin, &iconset).await?;
    let icns = resources.join("AppIcon.icns");
    run_tool(
        "iconutil",
        &[
            "-c",
            "icns",
            &iconset.to_string_lossy(),
            "-o",
            &icns.to_string_lossy(),
        ],
    )
    .await?;
    let _ = std::fs::remove_dir_all(&iconset);
    Ok(Some(String::from("AppIcon")))
}

async fn write_iconset(origin: &Path, iconset: &Path) -> Result<(), Failure> {
    for edge in [16u32, 32, 128, 256, 512] {
        for (scale, suffix) in [(1u32, String::new()), (2, String::from("@2x"))] {
            let pixels = edge * scale;
            let output = iconset.join(format!("icon_{edge}x{edge}{suffix}.png"));
            run_tool(
                "sips",
                &[
                    "-z",
                    &pixels.to_string(),
                    &pixels.to_string(),
                    &origin.to_string_lossy(),
                    "--out",
                    &output.to_string_lossy(),
                ],
            )
            .await?;
        }
    }
    Ok(())
}

fn info_plist(application: &NativeApplication, icon: Option<&str>) -> String {
    let icon = icon.map_or_else(String::new, |name| {
        format!("\n<key>CFBundleIconFile</key><string>{name}</string>")
    });
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleDevelopmentRegion</key><string>en</string>
<key>CFBundleDisplayName</key><string>{name}</string>
<key>CFBundleExecutable</key><string>{executable}</string>
<key>CFBundleIdentifier</key><string>{identifier}</string>{icon}
<key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
<key>CFBundleName</key><string>{name}</string>{location}
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>{version}</string>
<key>CFBundleVersion</key><string>{build}</string>
<key>LSMinimumSystemVersion</key><string>{minimum}</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
"#,
        name = xml_escape(&application.name),
        executable = application.executable_name,
        identifier = application.application_id,
        version = application.version,
        build = application.build,
        minimum = application
            .minimum_system
            .floor(tachyon_contracts::NativeTarget::Macos),
        icon = icon,
        location = application.permissions.location_plist_entries(
            &super::browser_api_permissions::BrowserApiPermissions::MACOS_LOCATION_KEYS
        ),
    )
}

fn window_controls(window: &super::config::WindowConfiguration) -> String {
    let granted = serde_json::to_string(&window.controls).unwrap_or_else(|_| String::from("[]"));
    let mut arms = format!(
        "        case \"window.granted\":\n            let request = tachyonParseJSONRequest(payload) ?? [:]\n            let granted: [String] = {granted}\n            let control = request[\"control\"] as? String ?? \"\"\n            return granted.contains(control) ? \"{{\\\"granted\\\":true}}\" : \"{{\\\"granted\\\":false}}\"\n"
    );
    if window.grants("minimize") {
        arms.push_str("        case \"window.minimize\":\n            window?.miniaturize(nil)\n            return \"{\\\"ok\\\":true}\"\n");
    }
    if window.grants("maximize") {
        arms.push_str("        case \"window.maximize\":\n            window?.zoom(nil)\n            return \"{\\\"ok\\\":true}\"\n");
    }
    if window.grants("fullscreen") {
        arms.push_str("        case \"window.fullscreen\":\n            window?.toggleFullScreen(nil)\n            return \"{\\\"ok\\\":true}\"\n");
    }
    if window.grants("close") {
        arms.push_str("        case \"window.close\":\n            window?.performClose(nil)\n            return \"{\\\"ok\\\":true}\"\n");
    }
    if window.grants("resize") {
        arms.push_str("        case \"window.resize\":\n            NativeBridge.resize(payload)\n            return \"{\\\"ok\\\":true}\"\n");
    }
    if window.grants("title") {
        arms.push_str("        case \"window.title\":\n            NativeBridge.retitle(payload)\n            return \"{\\\"ok\\\":true}\"\n");
    }
    arms.push_str(super::mac_os_title_bar_source::MacOsTitleBarSource::bridge_case(window));
    arms
}

fn with_window_settings(host: &str, window: &super::config::WindowConfiguration) -> String {
    host.replace("__WINDOW_WIDTH__", &window.width.to_string())
        .replace("__WINDOW_HEIGHT__", &window.height.to_string())
        .replace(
            "__WINDOW_MIN_WIDTH__",
            &window.min_width.unwrap_or(0).to_string(),
        )
        .replace(
            "__WINDOW_MIN_HEIGHT__",
            &window.min_height.unwrap_or(0).to_string(),
        )
        .replace("__WINDOW_CONTROLS__", &window_controls(window))
        .replace(
            "__NATIVE_TITLE_BAR_SETTINGS__",
            &super::mac_os_title_bar_source::MacOsTitleBarSource::settings(window),
        )
}

pub(super) fn swift_source(
    application: &NativeApplication,
    companion_call: &str,
    index: &NativeRouteIndex,
) -> String {
    let rust_shim = MacOsHostGenerator::rust_shim(companion_call);
    let host = MacOsHostGenerator::SWIFT_HOST
        .replace(
            "__TACHYON_JSON_REQUEST__",
            MacOsHostGenerator::APPLE_JSON_REQUEST,
        )
        .replace(
            "__NATIVE_BROWSER_APIS__",
            super::apple_browser_apis::AppleBrowserApis::SWIFT,
        );
    let framed = with_window_settings(&host, &application.window)
        .replace(
            "__NATIVE_TITLE_BAR__",
            &super::mac_os_title_bar_source::MacOsTitleBarSource::swift_types(),
        )
        .replace(
            "__NATIVE_SHIM__",
            &super::host::native_shim(&application.window, tachyon_contracts::NativeTarget::Macos),
        )
        .replace("__TACHYON_COMPANION_CALL__", companion_call)
        .replace("__TACHYON_COMPANION_EMIT__", companion_emit(companion_call))
        .replace(
            "__NATIVE_CAPABILITY_MODELS__",
            MacOsHostGenerator::capability_models(companion_call),
        )
        .replace(
            "__NATIVE_FILE_PICKER__",
            include_str!("capabilities/MacOsFilePicker.swift"),
        )
        .replace(
            "__NATIVE_FILE_PICKER_GRANTED__",
            &application
                .native_capabilities
                .iter()
                .any(|grant| grant == "filePicker")
                .to_string(),
        )
        .replace(
            "__NATIVE_CAPABILITY_REGISTER__",
            MacOsHostGenerator::capability_registration(companion_call),
        )
        .replace("__TACHYON_RUST_SHIM__", rust_shim)
        .replace(
            "__NATIVE_APPLICATION_START__",
            MacOsHostGenerator::application_start(companion_call),
        )
        .replace(
            "__NATIVE_APPLICATION_STOP__",
            MacOsHostGenerator::application_stop(companion_call),
        )
        .replace(
            "__NATIVE_APPLICATION_STATE__",
            include_str!("AppleApplicationState.swift"),
        )
        .replace("__NATIVE_MENUS__", include_str!("menus/MacOsMenus.swift"))
        .replace(
            "__NATIVE_MENU_MODELS__",
            concat!(
                include_str!("menus/MacOsMenuGroup.swift"),
                include_str!("menus/MacOsMenuDestination.swift"),
                include_str!("menus/MacOsMenuShortcut.swift"),
                include_str!("menus/MacOsMenuRoute.swift"),
                include_str!("menus/MacOsMenuNavigation.swift"),
                include_str!("menus/MacOsMenuWindow.swift"),
                include_str!("menus/MacOsMenuAction.swift"),
                include_str!("menus/MacOsMenuActions.swift"),
            ),
        )
        .replace(
            "__NATIVE_MENU_CALLS__",
            super::mac_os_menu_action_source::MacOsMenuActionSource::swift_calls(
                &application.menus,
                companion_call.contains("tacRustInvoke"),
            ),
        );
    MacOsHostGenerator::authored_values(&framed, application, index)
}

#[cfg(test)]
pub(super) const fn host_source() -> &'static str {
    MacOsHostGenerator::SWIFT_HOST
}

impl MacOsHostGenerator {
    fn authored_values(
        host: &str,
        application: &NativeApplication,
        index: &NativeRouteIndex,
    ) -> String {
        let app_name = quoted_string_escape(&application.name);
        let grants = quoted_string_escape(
            &serde_json::to_string(&application.native_capabilities)
                .unwrap_or_else(|_| String::from("[]")),
        );
        let menu_routes =
            super::mac_os_menu_source::MacOsMenuSource::declarations(&application.menus);
        let menu_actions = super::mac_os_menu_source::MacOsMenuSource::actions(&application.menus);
        Self::apple_authored_values(
            host,
            application,
            index,
            &[
                ("__APP_NAME__", &app_name),
                ("__NATIVE_GRANTS__", &grants),
                ("__NATIVE_MENU_ROUTES__", &menu_routes),
                ("__NATIVE_MENU_ACTIONS__", &menu_actions),
            ],
        )
    }

    pub(super) fn apple_authored_values(
        host: &str,
        application: &NativeApplication,
        index: &NativeRouteIndex,
        platform_values: &[(&str, &str)],
    ) -> String {
        let entry_document = quoted_string_escape(&index.entry_document);
        let entry_route = quoted_string_escape(&index.entry_route);
        let route_documents = super::routes::swift_routes(index);
        let app_type = format!("{}App", application.executable_name);
        let bundle_id = quoted_string_escape(&application.application_id);
        let policy = quoted_string_escape(&application.content_security_policy());
        let browser_api_grants =
            super::apple_browser_apis::AppleBrowserApis::swift_grants(&application.permissions);
        let mut values = vec![
            ("__ENTRY_DOCUMENT__", entry_document.as_str()),
            ("__ENTRY_ROUTE__", entry_route.as_str()),
            ("__ROUTE_DOCUMENTS__", route_documents.as_str()),
            ("__APP_TYPE__", app_type.as_str()),
            ("__BUNDLE_ID__", bundle_id.as_str()),
            ("__CONTENT_SECURITY_POLICY__", policy.as_str()),
            ("__NATIVE_BROWSER_API_GRANTS__", browser_api_grants.as_str()),
        ];
        values.extend_from_slice(platform_values);
        super::placeholder_fill::PlaceholderFill::once(host, &values)
    }

    fn rust_shim(call: &str) -> &'static str {
        if call.contains("tacRustInvoke") {
            return Self::RUST_SHIM;
        }
        ""
    }

    fn application_start(call: &str) -> &'static str {
        if call.contains("tacRustInvoke") {
            return "guard tacRustNativeApplicationStart() else { Telemetry.shared.record(\"application.start.failed\"); NSApplication.shared.terminate(nil); return }";
        }
        ""
    }

    fn application_stop(call: &str) -> &'static str {
        if call.contains("tacRustInvoke") {
            return "tacRustNativeApplicationShutdown()";
        }
        ""
    }

    fn capability_models(call: &str) -> &'static str {
        if call.contains("tacNativeInvoke") {
            return "";
        }
        concat!(
            include_str!("capabilities/FilePickerStatus.swift"),
            include_str!("capabilities/FilePickerResult.swift")
        )
    }

    fn capability_registration(call: &str) -> &'static str {
        if call.contains("tacNativeInvoke") {
            return "NativeCapabilities.call = { capability, payload in MainActor.assumeIsolated { NativeBridge.shared.handle(capability, payload) } }";
        }
        ""
    }

    pub(super) const RUST_SHIM: &str = r#"@_silgen_name("tac_native_invoke")
private func tacRustNativeInvoke(_ request: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tac_native_application_start")
private func tacRustNativeApplicationStart() -> Bool

@_silgen_name("tac_native_application_shutdown")
private func tacRustNativeApplicationShutdown()

@_silgen_name("tac_native_free")
private func tacRustNativeFree(_ answer: UnsafeMutablePointer<CChar>?)

@_silgen_name("tac_native_set_emit")
private func tacRustNativeSetEmit(_ emit: @convention(c) (UnsafePointer<CChar>?) -> Void)

@_silgen_name("tac_native_set_capability_adapter")
private func tacRustNativeSetCapabilityAdapter(
    _ call: @convention(c) (UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafeMutablePointer<Int>?) -> UnsafeMutablePointer<UInt8>?,
    _ free: @convention(c) (UnsafeMutablePointer<UInt8>?, Int) -> Void
)

private func tacRustCapabilityCall(
    _ capability: UnsafePointer<CChar>?, _ payload: UnsafePointer<CChar>?, _ length: UnsafeMutablePointer<Int>?
) -> UnsafeMutablePointer<UInt8>? {
    guard Thread.isMainThread, let capability, let payload, let length else { return nil }
    let answer = MainActor.assumeIsolated {
        NativeBridge.shared.handle(String(cString: capability), String(cString: payload))
    }
    let bytes = Array(answer.utf8)
    guard !bytes.isEmpty, bytes.count <= 1048576 else { return nil }
    let result = UnsafeMutablePointer<UInt8>.allocate(capacity: bytes.count)
    result.initialize(from: bytes, count: bytes.count)
    length.pointee = bytes.count
    return result
}

private func tacRustCapabilityFree(_ answer: UnsafeMutablePointer<UInt8>?, _ length: Int) {
    answer?.deallocate()
}

private func tacRustInvoke(_ request: String) -> String {
    request.withCString { pointer in
        guard let answer = tacRustNativeInvoke(pointer) else {
            return "{\"error\":\"The companion returned nothing.\"}"
        }
        defer { tacRustNativeFree(answer) }
        return String(cString: answer)
    }
}

private func tacRustEmit(_ payload: UnsafePointer<CChar>?) {
    guard let payload else { return }
    tachyonRelayPublish(String(cString: payload))
}"#;

    const SWIFT_HOST: &str = r##"import AppKit
import WebKit
import Foundation
import UniformTypeIdentifiers

__NATIVE_CAPABILITY_MODELS__
__NATIVE_FILE_PICKER__
__TACHYON_RUST_SHIM__
__TACHYON_JSON_REQUEST__
__NATIVE_BROWSER_APIS__
__NATIVE_MENUS__
__NATIVE_MENU_MODELS__
__NATIVE_MENU_CALLS__
__NATIVE_TITLE_BAR__
__NATIVE_APPLICATION_STATE__

private let tachyonEntryDocument = "__ENTRY_DOCUMENT__"
private let tachyonRouteDocuments: [String: String] = __ROUTE_DOCUMENTS__

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
    weak var window: NSWindow?
    weak var titleBar: MacOsTitleBar?

    func userContentController(
        _ controller: WKUserContentController,
        didReceive message: WKScriptMessage,
        replyHandler: @escaping (Any?, String?) -> Void
    ) {
        guard message.webView === webView, message.frameInfo.isMainFrame,
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



    static func resize(_ payload: String) {
        let data = payload.data(using: .utf8) ?? Data()
        let parsed = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] ?? [:]
        guard let window = NativeBridge.shared.window else { return }
        let width = (parsed["width"] as? NSNumber)?.doubleValue ?? Double(window.frame.width)
        let height = (parsed["height"] as? NSNumber)?.doubleValue ?? Double(window.frame.height)
        guard width.isFinite, height.isFinite,
              (64...16384).contains(width), (64...16384).contains(height) else { return }
        window.setContentSize(NSSize(width: width, height: height))
    }

    static func retitle(_ payload: String) {
        let data = payload.data(using: .utf8) ?? Data()
        let parsed = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] ?? [:]
        guard let title = parsed["title"] as? String else { return }
        let requested = String(title.prefix(256))
        NativeBridge.shared.window?.title = NativeBridge.shared.titleBar?.accessibleTitle(requested) ?? requested
    }

    fileprivate func handle(_ capability: String, _ payload: String) -> String {
        if capability.hasPrefix("window."), capability != "window.granted", window == nil {
            return "{\"error\":\"Native window unavailable.\"}"
        }
        if capability.hasPrefix("filePicker."), let request = tachyonParseJSONRequest(payload) {
            return MacOsFilePicker.shared.call(String(capability.dropFirst(11)), request)
        }
        switch capability {
__WINDOW_CONTROLS__
        case "companion.invoke":
            return __TACHYON_COMPANION_CALL__
        default:
            return "{\"ok\":false,\"error\":\"macos host answers companion.invoke, not '\(NativeBridge.safeName(capability))'\"}"
        }
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

private func tachyonRelayPublish(_ payload: String) {
    Task { @MainActor in
        NativeBridge.shared.webView?.evaluateJavaScript(
            "globalThis.__tachyonCompanionPublish(" + payload + ")")
    }
}

@MainActor
private final class AppDelegate: NSObject, NSApplicationDelegate, NSWindowDelegate, WKNavigationDelegate {
    private var window: NSWindow?
    private var webView: WKWebView?
    private var menus: MacOsMenus?
    private var titleBar: MacOsTitleBar?
    private var terminated = false

    func applicationDidFinishLaunching(_ notification: Notification) {
        MacOsFilePicker.shared.granted = __NATIVE_FILE_PICKER_GRANTED__
        AppleBrowserApis.shared.granted = __NATIVE_BROWSER_API_GRANTS__
        __NATIVE_CAPABILITY_REGISTER__
        __NATIVE_APPLICATION_START__
        Telemetry.shared.record("controller.created")
        openWindow()
        AppleApplicationState.shared.watch([
            NSApplication.didResignActiveNotification: "background",
            NSApplication.didHideNotification: "background",
            NSWindow.didMiniaturizeNotification: "background",
            NSApplication.didBecomeActiveNotification: "foreground",
            NSApplication.didUnhideNotification: "foreground",
            NSWindow.didDeminiaturizeNotification: "foreground",
        ])
        NSApplication.shared.activate(ignoringOtherApps: true)
        Telemetry.shared.record("bridge.ready")
    }

    private func openWindow() {
        let configuration = WKWebViewConfiguration()
        configuration.setURLSchemeHandler(TachyonAssetSchemeHandler.shared, forURLScheme: "tachyon-app")
        configuration.websiteDataStore = .default()
        let shim = WKUserScript(
            source: TACHYON_NATIVE_SHIM + "\nglobalThis.__tachyonNativeFilePicker = true; globalThis.__tachyonNativeGrants = __NATIVE_GRANTS__;",
            injectionTime: .atDocumentStart,
            forMainFrameOnly: true
        )
        configuration.userContentController.addUserScript(shim)
        configuration.userContentController.addScriptMessageHandler(
            NativeBridge.shared,
            contentWorld: .page,
            name: "tachyon"
        )

        let view = MacOsTitleBarWebView(frame: .zero, configuration: configuration)
        view.navigationDelegate = self
        view.setValue(false, forKey: "drawsBackground")
        view.setAccessibilityIdentifier("tachyon-application")
        webView = view
        NativeBridge.shared.webView = view
        __TACHYON_COMPANION_EMIT__

        let titleBar = MacOsTitleBar(__NATIVE_TITLE_BAR_SETTINGS__)
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: __WINDOW_WIDTH__, height: __WINDOW_HEIGHT__),
            styleMask: titleBar.styleMask,
            backing: .buffered,
            defer: false
        )
        window.title = "__APP_NAME__"
        window.contentView = view
        window.center()
        window.contentMinSize = NSSize(width: __WINDOW_MIN_WIDTH__, height: __WINDOW_MIN_HEIGHT__)
        window.collectionBehavior.insert(.fullScreenPrimary)
        window.isReleasedWhenClosed = false
        window.delegate = self
        titleBar.install(window, view)
        self.titleBar = titleBar
        NativeBridge.shared.titleBar = titleBar
        window.makeKeyAndOrderFront(nil)
        self.window = window
        NativeBridge.shared.window = window
        installMenus(view, window)

        AppleBrowserApis.shared.prepare(view) {
            view.load(URLRequest(url: URL(string: "tachyon-app://bundle/" + tachyonEntryDocument)!))
        }
    }

    private func installMenus(_ view: WKWebView?, _ window: NSWindow?) {
        let menus = MacOsMenus()
        let menuNavigation = MacOsMenuNavigation(view, window, tachyonRouteDocuments, __NATIVE_MENU_ROUTES__)
        let menuActions = MacOsMenuActions(view, window, __NATIVE_MENU_ACTIONS__, tachyonMenuReady, tachyonMenuPerform)
        menus.install(NSApplication.shared, window, "__APP_NAME__", menuNavigation, menuActions)
        self.menus = menus
    }

    func windowWillClose(_ notification: Notification) {
        guard let closing = notification.object as? NSWindow, closing === window else { return }
        tachyonDisposeCompanions()
        MacOsFilePicker.shared.disposeAll()
        webView?.navigationDelegate = nil
        webView?.stopLoading()
        closing.delegate = nil
        webView = nil
        window = nil
        titleBar = nil
        NativeBridge.shared.webView = nil
        NativeBridge.shared.window = nil
        NativeBridge.shared.titleBar = nil
        installMenus(nil, nil)
        Telemetry.shared.record("window.closed")
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        false
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        if window == nil && !terminated {
            openWindow()
            Telemetry.shared.record("window.reopened")
        }
        return true
    }

    func webView(_ webView: WKWebView, decidePolicyFor action: WKNavigationAction, decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
        guard let url = action.request.url else { decisionHandler(.cancel); return }
        if url.scheme == "tachyon-app" && url.host == "bundle" && url.port == nil && url.user == nil && url.password == nil { decisionHandler(.allow); return }
        decisionHandler(.cancel)
        let scheme = url.scheme?.lowercased() ?? ""
        let web = ["https", "http"].contains(scheme) && url.host?.isEmpty == false && url.user == nil && url.password == nil
        let mail = scheme == "mailto" && url.absoluteString.count > "mailto:".count
        if action.targetFrame?.isMainFrame != false && (web || mail) {
            NSWorkspace.shared.open(url)
        }
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        Telemetry.shared.record("controller.mounted")
        Telemetry.shared.record("controller.active")
        AppleApplicationState.shared.post()
    }

    func webView(_ webView: WKWebView, didStartProvisionalNavigation navigation: WKNavigation!) {
        tachyonDisposeCompanions()
        MacOsFilePicker.shared.disposeAll()
    }

    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
        Telemetry.shared.record("route.failed")
    }

    func applicationWillTerminate(_ notification: Notification) {
        guard !terminated else { return }
        terminated = true
        tachyonDisposeCompanions()
        MacOsFilePicker.shared.disposeAll()
        __NATIVE_APPLICATION_STOP__
        Telemetry.shared.record("controller.destroyed")
    }
}

private func tachyonDisposeCompanions() {
    let payload = "{\"op\":\"host-dispose\"}"
    _ = __TACHYON_COMPANION_CALL__
}

private let TACHYON_NATIVE_SHIM = #"""
__NATIVE_SHIM__
"""#

@main
private struct __APP_TYPE__ {
    static func main() {
        let application = NSApplication.shared
        let delegate = AppDelegate()
        application.delegate = delegate
        application.setActivationPolicy(.regular)
        withExtendedLifetime(delegate) { application.run() }
    }
}
"##;
}

#[cfg(test)]
mod tests {
    use super::{companion_call, info_plist, swift_source};
    use crate::native::config::NativeApplication;

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

    fn catalog_swift_source(call: &str) -> String {
        swift_source(&application(), call, &index())
    }

    #[test]
    fn the_host_is_a_window_around_the_applications_own_bundle() {
        let source = catalog_swift_source("tacNativeInvoke(payload)");
        assert!(source.contains("controller.created"));
        assert!(source.contains("controller.destroyed"));
        assert!(source.contains("TachyonAssetSchemeHandler"));
        assert!(source.contains("tachyon-app://bundle/"));
        assert!(source.contains("tacNativeInvoke(payload)"));
        assert!(source.contains("WKScriptMessageHandlerWithReply"));
        assert!(source.contains("didStartProvisionalNavigation"));
        assert!(source.contains("tachyonDisposeCompanions()"));
        assert!(
            source.contains(
                r#"["init", "get", "set", "call", "event", "dispose"].contains(operation)"#
            )
        );
        assert!(source.contains("__tachyonNativeHostCall"));
        assert!(!source.contains("case \"control.button\""));
        assert!(!source.contains("nodeView"));
    }

    #[test]
    fn a_host_without_a_companion_names_no_companion_symbol() {
        let source = catalog_swift_source(companion_call(None, None));
        assert!(!source.contains("tacNativeInvoke"));
        assert!(source.contains("has no native companion"));
    }

    #[test]
    fn mixed_companions_are_selected_without_constructing_and_both_are_disposed() {
        let path = std::path::Path::new("companion");
        let call = companion_call(Some(path), Some(path));
        assert!(call.contains("tacServiceKnown"));
        assert!(!call.contains("tacServiceMembers"));
        assert!(call.contains(r#"if request["op"] as? String == "host-dispose" { _ = tacNativeInvoke(payload); return tacRustInvoke(payload) }"#));
    }

    #[test]
    fn an_application_name_that_looks_like_a_placeholder_stays_text() {
        let mut named = application();
        named.name = String::from("__NATIVE_FILE_PICKER_GRANTED__ __ROUTE_DOCUMENTS__");
        let source = swift_source(&named, "tacNativeInvoke(payload)", &index());
        assert!(
            source
                .contains(r#"window.title = "__NATIVE_FILE_PICKER_GRANTED__ __ROUTE_DOCUMENTS__""#)
        );
        assert!(!source.contains("window.title = \"false"));
    }

    #[test]
    fn plist_declares_a_launchable_bundle() {
        let plist = info_plist(&application(), Some("AppIcon"));
        assert!(plist.contains("dev.tachyon.native-catalog"));
        assert!(plist.contains("NativeCatalog"));
        assert!(plist.contains("<key>LSMinimumSystemVersion</key><string>13.0</string>"));
    }

    #[test]
    fn a_raised_floor_reaches_the_plist_and_the_compiler_target() {
        let mut raised = application();
        raised.minimum_system = raised.minimum_system.with_floor(
            tachyon_contracts::NativeTarget::Macos,
            crate::native::system_version::SystemVersion::new(15, 2),
        );
        let plist = info_plist(&raised, None);
        assert!(plist.contains("<key>LSMinimumSystemVersion</key><string>15.2</string>"));
        assert!(
            crate::native::mac_os_executable::MacOsExecutable::swift_target(&raised, None)
                .ends_with("-apple-macosx15.2")
        );
    }

    #[test]
    fn swiftc_targets_the_minimum_system_on_every_slice() {
        let executable = crate::native::mac_os_executable::MacOsExecutable::swift_target;
        let slice = crate::native::mac_os_slice::MacOsSlice::build_machine();
        assert_eq!(
            crate::native::mac_os_architecture::MacOsArchitecture::build_machine().slices(),
            [slice]
        );
        assert_eq!(executable(&application(), None), slice.swift_target("13.0"));
        assert_eq!(
            executable(
                &application(),
                Some(crate::native::mac_os_slice::MacOsSlice::X64)
            ),
            "x86_64-apple-macosx13.0"
        );
    }

    #[test]
    fn plist_carries_the_version_and_the_build() {
        let mut application = application();
        application.version = String::from("2.1.0");
        application.build = 7;
        let plist = info_plist(&application, None);
        assert!(plist.contains("<key>CFBundleShortVersionString</key><string>2.1.0</string>"));
        assert!(plist.contains("<key>CFBundleVersion</key><string>7</string>"));
    }

    #[test]
    fn plist_carries_the_manifest_reason_for_location() {
        assert!(!info_plist(&application(), None).contains("NSLocation"));
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
        let plist = info_plist(&located, None);
        for key in [
            "NSLocationUsageDescription",
            "NSLocationWhenInUseUsageDescription",
        ] {
            assert!(plist.contains(&format!(
                "<key>{key}</key><string>Shows stores &amp; more</string>"
            )));
        }
    }

    #[test]
    fn the_host_sends_the_application_state_when_the_app_or_its_window_changes() {
        let source = catalog_swift_source("tacNativeInvoke(payload)");
        for (notification, state) in [
            ("NSApplication.didResignActiveNotification", "background"),
            ("NSApplication.didHideNotification", "background"),
            ("NSWindow.didMiniaturizeNotification", "background"),
            ("NSApplication.didBecomeActiveNotification", "foreground"),
            ("NSApplication.didUnhideNotification", "foreground"),
            ("NSWindow.didDeminiaturizeNotification", "foreground"),
        ] {
            assert!(
                source.contains(&format!("{notification}: \"{state}\"")),
                "{notification}"
            );
        }
        assert!(source.contains("#if os(iOS)"));
        assert!(source.contains("AppleApplicationState.shared.post()"));
        assert!(!source.contains("__NATIVE_APPLICATION_STATE__"));
    }
}
