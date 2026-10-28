use super::ordinary_browser_fixture::OrdinaryBrowserFixture;

struct DotnetHttpBrowserFixture;

impl DotnetHttpBrowserFixture {
    const ENVIRONMENT: [&'static str; 3] = [
        "TAC_DOTNET_HTTP_FIXTURE_OUT",
        "TAC_BIN",
        "TAC_DOTNET_HTTP_SOURCE_COMMIT",
    ];
    const CLIENT_FILES: [(&'static str, &'static str); 6] = [
        (
            "client/components/http/probe/tac.html",
            include_str!(
                "../../tests/fixtures/tac-dotnet-http/client/components/http/probe/tac.html"
            ),
        ),
        (
            "client/delegates/CustomHttpDelegate.cs",
            include_str!("../../tests/fixtures/tac-dotnet-http/client/delegates/CustomHttpDelegate.cs"),
        ),
        (
            "client/pages/tac.html",
            include_str!("../../tests/fixtures/tac-dotnet-http/client/pages/tac.html"),
        ),
        (
            "client/services/HttpProbeService.cs",
            include_str!(
                "../../tests/fixtures/tac-dotnet-http/client/services/HttpProbeService.cs"
            ),
        ),
        (
            "client/setups/BrowserHttpSetup.cs",
            include_str!("../../tests/fixtures/tac-dotnet-http/client/setups/BrowserHttpSetup.cs"),
        ),
        (
            "client/setups/OverrideHttpSetup.cs",
            include_str!("../../tests/fixtures/tac-dotnet-http/client/setups/OverrideHttpSetup.cs"),
        ),
    ];
    const SERVER_FILES: [(&'static str, &'static str); 2] = [
        (
            "server/controllers/ProbeController.js",
            include_str!(
                "../../tests/fixtures/tac-dotnet-http/server/controllers/ProbeController.js"
            ),
        ),
        (
            "server/controllers/SlowController.js",
            include_str!(
                "../../tests/fixtures/tac-dotnet-http/server/controllers/SlowController.js"
            ),
        ),
    ];

    async fn run() {
        let environment = OrdinaryBrowserFixture::read(Self::ENVIRONMENT);
        environment.validate();
        environment.stage(Self::CLIENT_FILES.into_iter().chain(Self::SERVER_FILES));
        environment.build().await;
        environment
            .browser(Self::ENVIRONMENT, "scripts/dotnet-http-browser-gate.mjs")
            .await;
    }
}

#[tokio::test]
#[ignore = "requires explicit matching compiler, isolated fixture/cache, .NET wasm-tools and installed Chromium"]
async fn dotnet_default_http_uses_real_yon_and_captured_browser_owners() {
    DotnetHttpBrowserFixture::run().await;
}
