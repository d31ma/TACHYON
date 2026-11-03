use super::ordinary_browser_fixture::OrdinaryBrowserFixture;
use std::path::Path;

struct FirstAppCsharpBrowserFixture;

impl FirstAppCsharpBrowserFixture {
    const ENVIRONMENT: [&'static str; 3] = [
        "TAC_DOTNET_HTTP_FIXTURE_OUT",
        "TAC_BIN",
        "TAC_DOTNET_HTTP_SOURCE_COMMIT",
    ];
    const CSHARP: &'static str = "cs";
    const FSHARP: &'static str = "fs";
    const STEP_FOUR: &'static str = "website/content/docs/first-app/call-the-server";
    const CLIENT_FILES: [&'static str; 6] = [
        "client/pages/hello/tac.html",
        "client/components/hello/card/tac.html",
        "client/services/CounterService.cs",
        "client/remotes/GreetingClient.cs",
        "client/models/Greeting.cs",
        "client/setups/GreetingSetup.cs",
    ];
    const STEP_THREE: &'static str = "website/content/docs/first-app/add-a-controller";
    const SERVER_FILES: [&'static str; 2] = [
        "server/controllers/GreetingController.js",
        "server/models/Greeting.ts",
    ];
    const STEP_FIVE: &'static str = "website/content/docs/first-app/add-tests";
    const TEST_FILES: [&'static str; 3] = [
        "client/delegates/GreetingFakeDelegate.cs",
        "client/tests/CounterServiceTest.cs",
        "client/tests/HelloTestSetup.cs",
    ];
    const TEST_UNIT: &'static str = "client/tests/CounterServiceTest.cs";
    const SERVER_TEST_FILES: [&'static str; 2] = [
        "server/tests/GreetingControllerTest.js",
        "server/tests/GreetingTestSetup.js",
    ];
    const GATE: &'static str = "scripts/first-app-csharp-browser-gate.mjs";

    fn in_language(files: &[&str], extension: &str) -> Vec<String> {
        let ending = format!(".{extension}");
        files
            .iter()
            .map(|file| file.replace(".cs", &ending))
            .collect()
    }

    fn stage_language(
        environment: &OrdinaryBrowserFixture,
        (root, files): (&str, &[&str]),
        extension: &str,
    ) {
        let written = Self::in_language(files, extension);
        let names: Vec<&str> = written.iter().map(String::as_str).collect();
        environment.stage_repo_files(root, &names);
    }

    fn stage(extension: &str) -> OrdinaryBrowserFixture {
        let environment = OrdinaryBrowserFixture::read(Self::ENVIRONMENT);
        environment.validate();
        Self::stage_language(
            &environment,
            (Self::STEP_FOUR, &Self::CLIENT_FILES),
            extension,
        );
        environment.stage_repo_files(Self::STEP_THREE, &Self::SERVER_FILES);
        environment
    }

    async fn browser(extension: &str) {
        let environment = Self::stage(extension);
        environment.build().await;
        environment.browser(Self::ENVIRONMENT, Self::GATE).await;
    }

    fn stage_tests(extension: &str) -> OrdinaryBrowserFixture {
        let environment = Self::stage(extension);
        Self::stage_language(
            &environment,
            (Self::STEP_FIVE, &Self::TEST_FILES),
            extension,
        );
        environment.stage_repo_files(Self::STEP_FIVE, &Self::SERVER_TEST_FILES);
        environment
    }

    async fn after_tests_browser(extension: &str) {
        let environment = Self::stage_tests(extension);
        environment.build().await;
        environment.browser(Self::ENVIRONMENT, Self::GATE).await;
    }

    fn tests(extension: &str) -> Result<(), String> {
        let environment = Self::stage_tests(extension);
        let unit = Self::in_language(&[Self::TEST_UNIT], extension).concat();
        let report = crate::client_tests::run(&environment.root, Path::new(&unit))
            .map_err(|error| error.to_string())?;
        assert!(report.failure.is_none(), "{report:?}");
        assert_eq!(report.cases.len(), 3, "{report:?}");
        assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
        Ok(())
    }
}

#[tokio::test]
#[ignore = "requires matching registered-record compiler, isolated cache/root, .NET wasm-tools and installed browser; canonical C# tutorial"]
async fn first_app_csharp_cumulative_step_four_uses_published_yon() {
    FirstAppCsharpBrowserFixture::browser(FirstAppCsharpBrowserFixture::CSHARP).await;
}

#[tokio::test]
#[ignore = "requires explicitly staged first-app C# build and matching compiler; no SDK rebuild"]
async fn first_app_csharp_existing_build_uses_published_yon() {
    let environment = OrdinaryBrowserFixture::read(FirstAppCsharpBrowserFixture::ENVIRONMENT);
    environment.validate();
    assert!(
        environment.root.join("dist").is_dir(),
        "built fixture required"
    );
    environment
        .browser(
            FirstAppCsharpBrowserFixture::ENVIRONMENT,
            FirstAppCsharpBrowserFixture::GATE,
        )
        .await;
}

#[test]
#[ignore = "requires .NET SDK and explicit separate isolated Test root/cache; executes canonical C# Step5 typed Fake with caller-owned Context"]
fn first_app_csharp_cumulative_step_five_uses_owned_typed_fake() -> Result<(), String> {
    FirstAppCsharpBrowserFixture::tests(FirstAppCsharpBrowserFixture::CSHARP)
}

#[tokio::test]
#[ignore = "requires separate staged canonical Step5 root and matching compiler; qualifies ordinary post-Test browser integration"]
async fn first_app_csharp_post_step_five_browser_uses_published_yon() {
    FirstAppCsharpBrowserFixture::after_tests_browser(FirstAppCsharpBrowserFixture::CSHARP).await;
}

#[tokio::test]
#[ignore = "requires matching compiler, isolated cache/root, .NET wasm-tools and installed browser; canonical F# tutorial"]
async fn first_app_fsharp_cumulative_step_four_uses_published_yon() {
    FirstAppCsharpBrowserFixture::browser(FirstAppCsharpBrowserFixture::FSHARP).await;
}

#[test]
#[ignore = "requires .NET SDK and explicit separate isolated Test root/cache; executes canonical F# Step5 typed Fake with caller-owned Context"]
fn first_app_fsharp_cumulative_step_five_uses_owned_typed_fake() -> Result<(), String> {
    FirstAppCsharpBrowserFixture::tests(FirstAppCsharpBrowserFixture::FSHARP)
}

#[tokio::test]
#[ignore = "requires separate staged canonical F# Step5 root and matching compiler; qualifies ordinary post-Test browser integration"]
async fn first_app_fsharp_post_step_five_browser_uses_published_yon() {
    FirstAppCsharpBrowserFixture::after_tests_browser(FirstAppCsharpBrowserFixture::FSHARP).await;
}
