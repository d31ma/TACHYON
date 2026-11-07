use super::ordinary_browser_fixture::OrdinaryBrowserFixture;

struct DotnetTaskCollectionBrowserFixture;

impl DotnetTaskCollectionBrowserFixture {
    const ENVIRONMENT: [&'static str; 3] = [
        "TAC_DOTNET_HTTP_FIXTURE_OUT",
        "TAC_BIN",
        "TAC_DOTNET_HTTP_SOURCE_COMMIT",
    ];
    const SOURCE: &'static str = "crates/tachyon-core/tests/fixtures/tac-csharp-task-collections";
    const FILES: [&'static str; 9] = [
        "client/models/MessageItem.cs",
        "client/models/MessagePage.cs",
        "client/remotes/CollectionClient.cs",
        "client/services/CollectionService.cs",
        "client/setups/CollectionSetup.cs",
        "client/manifest.json",
        "client/pages/tac.html",
        "client/components/collection/tac.html",
        "server/controllers/CollectionController.js",
    ];
    const GATE: &'static str = "scripts/dotnet-task-collection-browser-gate.mjs";

    async fn run() {
        let environment = OrdinaryBrowserFixture::read(Self::ENVIRONMENT);
        environment.validate();
        environment.stage_repo_files(Self::SOURCE, &Self::FILES);
        environment.build().await;
        environment.browser(Self::ENVIRONMENT, Self::GATE).await;
    }

    async fn existing() {
        let environment = OrdinaryBrowserFixture::read(Self::ENVIRONMENT);
        environment.validate();
        assert!(
            environment.root.join("dist").is_dir(),
            "built fixture required"
        );
        environment.browser(Self::ENVIRONMENT, Self::GATE).await;
    }
}

#[tokio::test]
#[ignore = "requires matching collection-model compiler, explicit isolated fixture/cache, .NET wasm-tools and installed browser"]
async fn dotnet_registered_task_collection_fields_use_real_yon_and_owned_disposal() {
    DotnetTaskCollectionBrowserFixture::run().await;
}

#[tokio::test]
#[ignore = "requires explicit already-built collection fixture and matching compiler; no SDK rebuild"]
async fn dotnet_registered_task_collection_existing_build_uses_real_yon_and_owned_disposal() {
    DotnetTaskCollectionBrowserFixture::existing().await;
}
