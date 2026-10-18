use std::path::Path;

struct CsharpTaskTestFixture;

impl CsharpTaskTestFixture {
    const COLLECTION_ROOT: &'static str = "tests/fixtures/tac-csharp-task-collections";
    const COLLECTION_FILES: [&'static str; 9] = [
        "client/models/MessageItem.cs",
        "client/models/MessagePage.cs",
        "client/remotes/CollectionClient.cs",
        "client/services/CollectionService.cs",
        "client/setups/CollectionSetup.cs",
        "client/setups/CollectionFakeSetup.cs",
        "client/helpers/CollectionAnswerHelper.cs",
        "client/tests/CollectionTestSetup.cs",
        "client/tests/CollectionServiceTest.cs",
    ];

    const RESULT_ROOT: &'static str = "tests/fixtures/tac-csharp-task-results";
    const RESULT_FILES: [&'static str; 10] = [
        "client/models/ShelfItem.cs",
        "client/models/ShelfDetail.cs",
        "client/models/Shelf.cs",
        "client/remotes/ResultClient.cs",
        "client/services/ResultService.cs",
        "client/setups/ResultSetup.cs",
        "client/setups/ResultFakeSetup.cs",
        "client/helpers/ResultAnswerHelper.cs",
        "client/tests/ResultTestSetup.cs",
        "client/tests/ResultServiceTest.cs",
    ];

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

    fn collection(root: &Path) -> Result<(), String> {
        for relative in Self::COLLECTION_FILES {
            Self::write_from(root, relative, Self::COLLECTION_ROOT)?;
        }
        Ok(())
    }

    fn results(root: &Path) -> Result<(), String> {
        for relative in Self::RESULT_FILES {
            Self::write_from(root, relative, Self::RESULT_ROOT)?;
        }
        Ok(())
    }

    fn write(root: &Path, relative: &str) -> Result<(), String> {
        Self::write_from(root, relative, Self::ROOT)
    }

    fn write_from(root: &Path, relative: &str, source_root: &str) -> Result<(), String> {
        let path = root.join(relative);
        let parent = path
            .parent()
            .ok_or_else(|| "fixture source parent".to_owned())?;
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(source_root)
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

#[test]
#[ignore = "requires installed .NET SDK; genuine supplied-Context typed Fake list-record decoding without browser transport"]
fn csharp_registered_task_collection_runs_genuine_authored_test_fakes() -> Result<(), String> {
    let root = tempfile::tempdir().map_err(|error| error.to_string())?;
    CsharpTaskTestFixture::collection(root.path())?;
    let report = super::run(
        root.path(),
        Path::new("client/tests/CollectionServiceTest.cs"),
    )
    .map_err(|error| error.to_string())?;
    assert!(report.failure.is_none(), "{report:?}");
    assert_eq!(report.cases.len(), 11);
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
    Ok(())
}

#[test]
fn csharp_registered_task_collection_pure_context_captures_both_models_without_browser_events()
-> Result<(), String> {
    let root = tempfile::tempdir().map_err(|error| error.to_string())?;
    CsharpTaskTestFixture::collection(root.path())?;
    let context = crate::route_types::Converters::csharp_tac_test_text(root.path())?;
    assert!(context.contains("typeof(Models.MessagePage)"));
    assert!(context.contains("typeof(Models.MessageItem)"));
    assert!(!context.contains("typeof(PointerEvent)"));
    assert!(!context.contains("CaptureHttpTransport"));
    Ok(())
}

#[test]
#[ignore = "requires installed .NET SDK; typed Fake lists, arrays, plain values, a nested record and a ValueTask without browser transport"]
fn csharp_task_lists_values_nested_records_and_value_tasks_run_genuine_authored_test_fakes()
-> Result<(), String> {
    let root = tempfile::tempdir().map_err(|error| error.to_string())?;
    CsharpTaskTestFixture::results(root.path())?;
    let report = super::run(root.path(), Path::new("client/tests/ResultServiceTest.cs"))
        .map_err(|error| error.to_string())?;
    assert!(report.failure.is_none(), "{report:?}");
    assert_eq!(report.cases.len(), 4);
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
    Ok(())
}
