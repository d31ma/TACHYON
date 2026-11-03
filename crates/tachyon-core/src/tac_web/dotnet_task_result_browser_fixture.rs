use super::ordinary_browser_fixture::OrdinaryBrowserFixture;

struct DotnetTaskResultBrowserFixture;

impl DotnetTaskResultBrowserFixture {
    const ENVIRONMENT: [&'static str; 3] = [
        "TAC_DOTNET_HTTP_FIXTURE_OUT",
        "TAC_BIN",
        "TAC_DOTNET_HTTP_SOURCE_COMMIT",
    ];
    const SOURCE: &'static str = "crates/tachyon-core/tests/fixtures/tac-csharp-task-results";
    const FILES: [&'static str; 10] = [
        "client/models/ShelfItem.cs",
        "client/models/ShelfDetail.cs",
        "client/models/Shelf.cs",
        "client/remotes/ResultClient.cs",
        "client/services/ResultService.cs",
        "client/setups/ResultSetup.cs",
        "client/manifest.json",
        "client/pages/tac.html",
        "client/components/result/tac.html",
        "server/controllers/ResultController.js",
    ];
    const GATE: &'static str = "scripts/dotnet-task-result-browser-gate.mjs";

    async fn run() {
        let environment = OrdinaryBrowserFixture::read(Self::ENVIRONMENT);
        environment.validate();
        environment.stage_repo_files(Self::SOURCE, &Self::FILES);
        environment.build().await;
        environment.browser(Self::ENVIRONMENT, Self::GATE).await;
    }
}

#[tokio::test]
#[ignore = "requires matching compiler, explicit isolated fixture/cache, .NET wasm-tools and installed browser"]
async fn dotnet_task_lists_values_nested_records_and_value_tasks_use_real_yon() {
    DotnetTaskResultBrowserFixture::run().await;
}
