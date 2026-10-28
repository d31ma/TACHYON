use super::ordinary_browser_fixture::OrdinaryBrowserFixture;

struct FsharpHttpBrowserFixture;

impl FsharpHttpBrowserFixture {
    const ENVIRONMENT: [&'static str; 3] = [
        "TAC_DOTNET_HTTP_FIXTURE_OUT",
        "TAC_BIN",
        "TAC_DOTNET_HTTP_SOURCE_COMMIT",
    ];
    const CLIENT_ROOT: &'static str = "crates/tachyon-core/tests/fixtures/tac-fsharp-http";
    const CLIENT_FILES: [&'static str; 6] = [
        "client/components/http/probe/tac.html",
        "client/delegates/CustomHttpDelegate.fs",
        "client/pages/tac.html",
        "client/services/HttpProbeService.fs",
        "client/setups/BrowserHttpSetup.fs",
        "client/setups/OverrideHttpSetup.fs",
    ];
    const SERVER_ROOT: &'static str = "crates/tachyon-core/tests/fixtures/tac-dotnet-http";
    const SERVER_FILES: [&'static str; 2] = [
        "server/controllers/ProbeController.js",
        "server/controllers/SlowController.js",
    ];

    async fn run() {
        let fixture = OrdinaryBrowserFixture::read(Self::ENVIRONMENT);
        fixture.validate();
        fixture.stage_repo_files(Self::CLIENT_ROOT, &Self::CLIENT_FILES);
        fixture.stage_repo_files(Self::SERVER_ROOT, &Self::SERVER_FILES);
        fixture.build().await;
        fixture
            .browser(Self::ENVIRONMENT, "scripts/fsharp-http-browser-gate.mjs")
            .await;
    }
}

#[tokio::test]
#[ignore = "requires explicit matching compiler, isolated fixture/cache, .NET wasm-tools and installed browser; actual F# Task HTTP owner proof"]
async fn fsharp_browser_http_uses_existing_task_host_and_real_yon() {
    FsharpHttpBrowserFixture::run().await;
}
