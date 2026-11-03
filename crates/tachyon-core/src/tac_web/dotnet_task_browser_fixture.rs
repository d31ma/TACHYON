use super::ordinary_browser_fixture::OrdinaryBrowserFixture;

struct DotnetTaskBrowserFixture {
    environment: OrdinaryBrowserFixture,
}

impl DotnetTaskBrowserFixture {
    const ENVIRONMENT: [&'static str; 3] = [
        "TAC_DOTNET_TASK_FIXTURE_OUT",
        "TAC_BIN",
        "TAC_DOTNET_TASK_SOURCE_COMMIT",
    ];
    const FILES: [(&'static str, &'static str); 3] = [
        (
            "client/services/DelayTaskService.cs",
            include_str!(
                "../../tests/fixtures/tac-dotnet-tasks/client/services/DelayTaskService.cs"
            ),
        ),
        (
            "client/pages/tac.html",
            include_str!("../../tests/fixtures/tac-dotnet-tasks/client/pages/tac.html"),
        ),
        (
            "client/components/task/probe/tac.html",
            include_str!(
                "../../tests/fixtures/tac-dotnet-tasks/client/components/task/probe/tac.html"
            ),
        ),
    ];

    async fn run() {
        let fixture = Self::read();
        fixture.environment.validate();
        fixture.stage();
        fixture.environment.build().await;
        fixture.browser().await;
    }

    fn read() -> Self {
        Self {
            environment: OrdinaryBrowserFixture::read(Self::ENVIRONMENT),
        }
    }

    fn stage(&self) {
        self.environment.stage(Self::FILES);
    }

    async fn browser(&self) {
        self.environment
            .browser(Self::ENVIRONMENT, "scripts/dotnet-task-browser-gate.mjs")
            .await;
    }
}

#[tokio::test]
#[ignore = "requires an explicit matching compiler, isolated fixture/cache, .NET wasm-tools and installed Chromium"]
async fn dotnet_tasks_use_the_ordinary_compiler_and_real_browser_continuations() {
    DotnetTaskBrowserFixture::run().await;
}
