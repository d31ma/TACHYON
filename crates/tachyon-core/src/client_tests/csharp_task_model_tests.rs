use std::path::Path;

struct CsharpTaskTestFixture;

impl CsharpTaskTestFixture {
    const ROOT: &'static str = "tests/fixtures/tac-csharp-task-models";
    const FILES: [&'static str; 12] = [
        "client/models/Greeting.cs",
        "client/models/NullableGreeting.cs",
        "client/remotes/GreetingClient.cs",
        "client/services/GreetingService.cs",
        "client/setups/GreetingSetup.cs",
        "client/setups/GreetingFakeSetup.cs",
        "client/setups/GreetingBrokenSetup.cs",
        "client/setups/GreetingNullSetup.cs",
        "client/setups/GreetingExtraSetup.cs",
        "client/helpers/GreetingAnswerHelper.cs",
        "client/tests/GreetingTestSetup.cs",
        "client/tests/GreetingServiceTest.cs",
    ];

    fn stage(root: &Path) -> Result<(), String> {
        for relative in Self::FILES {
            Self::write(root, relative)?;
        }
        Ok(())
    }

    fn write(root: &Path, relative: &str) -> Result<(), String> {
        let path = root.join(relative);
        let parent = path
            .parent()
            .ok_or_else(|| "fixture source parent".to_owned())?;
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(Self::ROOT)
            .join(relative);
        std::fs::copy(source, path).map_err(|error| error.to_string())?;
        Ok(())
    }
}

#[test]
#[ignore = "requires installed .NET SDK; executes genuine typed client Test/Fake model decoding and context disposal without browser HTTP"]
fn csharp_registered_task_model_runs_genuine_authored_test_fakes() -> Result<(), String> {
    let root = tempfile::tempdir().map_err(|error| error.to_string())?;
    CsharpTaskTestFixture::stage(root.path())?;
    let report = super::run(
        root.path(),
        Path::new("client/tests/GreetingServiceTest.cs"),
    )
    .map_err(|error| error.to_string())?;
    assert!(report.failure.is_none(), "{report:?}");
    assert_eq!(report.cases.len(), 4);
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
    Ok(())
}

#[test]
fn csharp_pure_test_model_context_omits_ambient_browser_events() -> Result<(), String> {
    let root = tempfile::tempdir().map_err(|error| error.to_string())?;
    CsharpTaskTestFixture::stage(root.path())?;
    let context = crate::route_types::Converters::csharp_tac_test_text(root.path())?;
    assert!(context.contains("typeof(Models.Greeting)"));
    assert!(context.contains("typeof(Models.NullableGreeting)"));
    for event in crate::route_types::WebEventModels::names() {
        assert!(!context.contains(&format!("typeof({event})")));
    }
    Ok(())
}
