use super::ordinary_browser_fixture::OrdinaryBrowserFixture;

struct DotnetTaskModelBrowserFixture;

impl DotnetTaskModelBrowserFixture {
    const ENVIRONMENT: [&'static str; 3] = [
        "TAC_DOTNET_HTTP_FIXTURE_OUT",
        "TAC_BIN",
        "TAC_DOTNET_HTTP_SOURCE_COMMIT",
    ];
    const SOURCE: &'static str = "crates/tachyon-core/tests/fixtures/tac-csharp-task-models";
    const APPLICATION_FILES: [&'static str; 5] = [
        "client/remotes/GreetingClient.cs",
        "client/services/GreetingService.cs",
        "client/setups/GreetingSetup.cs",
        "client/pages/tac.html",
        "client/components/greeting/tac.html",
    ];
    const MODEL_FILES: [&'static str; 2] = [
        "client/models/Greeting.cs",
        "client/models/NullableGreeting.cs",
    ];
    const YON_SOURCE: &'static str =
        "crates/tachyon-core/tests/fixtures/tac-csharp-task-model-browser";
    const YON_FILES: [&'static str; 2] = [
        "client/manifest.json",
        "server/controllers/GreetingController.js",
    ];
    const GATE: &'static str = "scripts/dotnet-task-model-browser-gate.mjs";

    async fn run() {
        let environment = OrdinaryBrowserFixture::read(Self::ENVIRONMENT);
        environment.validate();
        environment.stage_repo_files(Self::SOURCE, &Self::APPLICATION_FILES);
        environment.stage_repo_files(Self::SOURCE, &Self::MODEL_FILES);
        environment.stage_repo_files(Self::YON_SOURCE, &Self::YON_FILES);
        environment.build().await;
        environment.browser(Self::ENVIRONMENT, Self::GATE).await;
    }
}

#[tokio::test]
#[ignore = "requires matching registered-record compiler, explicit isolated fixture/cache, .NET wasm-tools and installed browser"]
async fn dotnet_registered_task_models_use_real_yon_and_owned_client_disposal() {
    DotnetTaskModelBrowserFixture::run().await;
}

#[tokio::test]
#[ignore = "requires explicit already-built registered-record fixture and matching compiler; no SDK rebuild"]
async fn dotnet_registered_task_models_existing_build_uses_real_yon_and_owned_disposal() {
    let environment = OrdinaryBrowserFixture::read(DotnetTaskModelBrowserFixture::ENVIRONMENT);
    environment.validate();
    assert!(
        environment.root.join("dist").is_dir(),
        "built fixture required"
    );
    environment
        .browser(
            DotnetTaskModelBrowserFixture::ENVIRONMENT,
            DotnetTaskModelBrowserFixture::GATE,
        )
        .await;
}
