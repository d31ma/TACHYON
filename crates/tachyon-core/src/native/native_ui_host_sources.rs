use super::native_ui_problem::NativeUiProblem;
use crate::Failure;

pub(super) struct NativeUiHostSources;

impl NativeUiHostSources {
    const OWNER: &str = r"    private let liveStyleHost: NativeUiStyleHost
    private var fixtureLaunched = false
    private let fixtureFrame = NativeUiHostFrameFixture()

    init(_ installed: NativeUiRuntimeGeneration) {
        liveStyleHost = NativeUiStyleHost(installed)
        super.init()
    }
";
    const NATIVE_REPLY: &str = "const reply=await globalThis.__tachyonNativeHostCall('companion.invoke',JSON.stringify({route:'/',service:'liveCounter',owner:'6F9619FF-8B86-D011-B42D-00C04FC964FF',op:'call',name:'read',args:[],fixtureDelay:true})); if(JSON.parse(reply).value!==1)throw new Error('Native state changed'); return 'reply-kept'";

    pub(super) fn host(original: &str) -> Result<String, Failure> {
        let body = Self::without_main(original)?;
        let owned = Self::owner(&body)?;
        let configured = Self::configuration(&owned)?;
        let bound = Self::binding(&configured)?;
        let launched = Self::launch(&bound)?;
        let navigation = Self::navigation(&launched)?;
        let termination = Self::termination(&navigation)?;
        let replies = Self::replies(&termination)?;
        Ok(Self::fragments(&replies))
    }

    pub(super) fn caller() -> Result<String, Failure> {
        let caller = include_str!("live/NativeUiStyleCheck.fixture.swift");
        let pending = Self::replace(
            caller,
            "await new Promise(resolve => setTimeout(resolve, 250)); return 'reply-kept'",
            Self::NATIVE_REPLY,
        )?;
        let gated = Self::replace(
            &pending,
            "        let ticket = try await scene.session.prepare(candidate)\n        try await scene.session.commit(ticket)\n        try await committed(scene, marker, identity)",
            "        try await scene.waitForNativeReply()\n        let ticket = try await scene.session.prepare(candidate)\n        try await scene.session.commit(ticket)\n        try await committed(scene, marker, identity)\n        try scene.releaseNativeReply()",
        )?;
        let routed = Self::replace(
            &gated,
            "        scene.window.close()",
            "        try await scene.routes(candidate)\n        scene.window.close()",
        )?;
        Self::replace(
            &routed,
            "        print(\"native-ui-styles-retained-rollback-passed\")",
            "        try NativeUiHostRoutesFixture.check(assets)\n        print(\"native-ui-styles-retained-rollback-passed\")",
        )
    }

    fn without_main(source: &str) -> Result<String, Failure> {
        source
            .rsplit_once("\n@main\n")
            .map(|(body, _)| body.to_owned())
            .ok_or_else(|| NativeUiProblem::failure("Generated host entry point is missing."))
    }

    fn owner(source: &str) -> Result<String, Failure> {
        Self::replace(
            source,
            "private final class AppDelegate: NSObject, NSApplicationDelegate, WKNavigationDelegate {\n",
            &format!(
                "private final class AppDelegate: NSObject, NSApplicationDelegate, WKNavigationDelegate {{\n{}",
                Self::OWNER
            ),
        )
    }

    fn configuration(source: &str) -> Result<String, Failure> {
        Self::replace(
            source,
            "        configuration.setURLSchemeHandler(TachyonAssetSchemeHandler.shared, forURLScheme: \"tachyon-app\")",
            "        liveStyleHost.configure(configuration, tachyonDocument, NativeUiHostRoutesFixture.selectedDocument)\n        fixtureFrame.configure(configuration)",
        )
    }

    fn binding(source: &str) -> Result<String, Failure> {
        Self::replace(
            source,
            "        webView = view\n",
            "        webView = view\n        do { try liveStyleHost.bind(view) } catch { fatalError(\"Trusted host fixture binding failed: \\(error)\") }\n",
        )
    }

    fn launch(source: &str) -> Result<String, Failure> {
        Self::replace(
            source,
            "    func applicationDidFinishLaunching(_ notification: Notification) {\n",
            "    func applicationDidFinishLaunching(_ notification: Notification) {\n        guard !fixtureLaunched else { return }\n        fixtureLaunched = true\n",
        )
    }

    fn navigation(source: &str) -> Result<String, Failure> {
        Self::replace(
            source,
            "    func webView(_ webView: WKWebView, didStartProvisionalNavigation navigation: WKNavigation!) {\n",
            "    func webView(_ webView: WKWebView, didStartProvisionalNavigation navigation: WKNavigation!) {\n        liveStyleHost.invalidate()\n",
        )
    }

    fn termination(source: &str) -> Result<String, Failure> {
        Self::replace(
            source,
            "    func applicationWillTerminate(_ notification: Notification) {\n",
            "    func applicationWillTerminate(_ notification: Notification) {\n        liveStyleHost.invalidate()\n",
        )
    }

    fn replies(source: &str) -> Result<String, Failure> {
        Self::replace(
            source,
            "        replyHandler(handle(capability, canonicalPayload), nil)",
            "        NativeUiHostReplyFixture.deliver(handle(capability, canonicalPayload), request, replyHandler)",
        )
    }

    fn fragments(source: &str) -> String {
        [
            source,
            include_str!("live/NativeUiHostAccess.fixture.swift"),
            include_str!("live/NativeUiHostReply.fixture.swift"),
            include_str!("live/NativeUiHostRoutes.fixture.swift"),
            include_str!("live/NativeUiHostFrame.fixture.swift"),
            include_str!("live/NativeUiHostScene.fixture.swift"),
        ]
        .join("\n")
    }

    fn replace(source: &str, marker: &str, replacement: &str) -> Result<String, Failure> {
        NativeUiProblem::require(
            source.matches(marker).count() == 1,
            "Trusted host fixture requires exactly one generated ownership seam.",
        )?;
        Ok(source.replace(marker, replacement))
    }
}
