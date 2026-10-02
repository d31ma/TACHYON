use super::BuildOptions;
use super::component_script::ComponentScript;
use super::service_checks::ServiceChecks;
use super::site_build::SiteBuild;
use std::path::Path;

struct CsharpModelStagingFixture;

impl CsharpModelStagingFixture {
    const ROOT: &'static str = "tests/fixtures/tac-csharp-task-models";
    const FILES: [&'static str; 7] = [
        "client/models/Greeting.cs",
        "client/models/NullableGreeting.cs",
        "client/remotes/GreetingClient.cs",
        "client/services/GreetingService.cs",
        "client/setups/GreetingSetup.cs",
        "client/pages/tac.html",
        "client/components/greeting/tac.html",
    ];
    const SERVICE: &'static str = "client/services/GreetingService.cs";

    fn stage(root: &Path) -> Result<(), String> {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(Self::ROOT);
        for relative in Self::FILES {
            let source =
                std::fs::read(fixture.join(relative)).map_err(|error| error.to_string())?;
            Self::write(root, relative, &source)?;
        }
        Ok(())
    }

    fn write(root: &Path, relative: &str, source: &[u8]) -> Result<(), String> {
        let path = root.join(relative);
        let parent = path.parent().ok_or_else(|| "staging parent".to_owned())?;
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        std::fs::write(path, source).map_err(|error| error.to_string())
    }

    fn unknown(root: &Path) -> Result<(), String> {
        let path = "client/remotes/GreetingClient.cs";
        let source = std::fs::read_to_string(root.join(path)).map_err(|error| error.to_string())?;
        let source = source.replace("Models.Greeting>", "Models.UnknownGreeting>");
        Self::write(root, path, source.as_bytes())
    }
}

#[test]
fn csharp_task_model_cli_discovery_stages_registered_browser_dto_before_sdk() -> Result<(), String>
{
    let root = tempfile::tempdir().map_err(|error| error.to_string())?;
    CsharpModelStagingFixture::stage(root.path())?;
    let project =
        crate::ProjectDiscovery::discover(root.path()).map_err(|error| error.to_string())?;
    let build = SiteBuild::discover(&project, &BuildOptions::default(), None)
        .map_err(|error| error.to_string())?;
    let programs = build
        .compile_route_programs()
        .map_err(|error| error.to_string())?;
    assert!(programs.contains_key("/"));
    Ok(())
}

#[test]
fn csharp_task_model_cli_discovery_refuses_unregistered_result_early() -> Result<(), String> {
    let root = tempfile::tempdir().map_err(|error| error.to_string())?;
    CsharpModelStagingFixture::stage(root.path())?;
    CsharpModelStagingFixture::unknown(root.path())?;
    let project =
        crate::ProjectDiscovery::discover(root.path()).map_err(|error| error.to_string())?;
    let failure = SiteBuild::discover(&project, &BuildOptions::default(), None)
        .err()
        .ok_or_else(|| "unknown DTO must fail before SDK".to_owned())?;
    assert!(failure.to_string().contains("UnknownGreeting"));
    assert!(
        failure
            .diagnostics()
            .iter()
            .any(|item| !item.spans.is_empty())
    );
    Ok(())
}

#[test]
fn csharp_task_model_native_target_retains_original_result_refusal() -> Result<(), String> {
    let root = tempfile::tempdir().map_err(|error| error.to_string())?;
    CsharpModelStagingFixture::stage(root.path())?;
    let project =
        crate::ProjectDiscovery::discover(root.path()).map_err(|error| error.to_string())?;
    let failure = SiteBuild::discover(
        &project,
        &BuildOptions::default(),
        Some(tachyon_contracts::NativeTarget::Windows),
    )
    .err()
    .ok_or_else(|| "native DTO Task remains unsupported".to_owned())?;
    assert!(failure.to_string().contains("Task<BehaviorJSON>"));
    let source = std::fs::read_to_string(root.path().join(CsharpModelStagingFixture::SERVICE))
        .map_err(|error| error.to_string())?;
    assert!(
        ServiceChecks::check_native_source(
            root.path(),
            Path::new(CsharpModelStagingFixture::SERVICE),
            &source
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn csharp_task_model_component_preparation_uses_the_captured_inventory() -> Result<(), String> {
    let root = tempfile::tempdir().map_err(|error| error.to_string())?;
    CsharpModelStagingFixture::stage(root.path())?;
    let source = std::fs::read(root.path().join(CsharpModelStagingFixture::SERVICE))
        .map_err(|error| error.to_string())?;
    let prepared =
        ComponentScript::prepare_captured(root.path(), source, CsharpModelStagingFixture::SERVICE)
            .map_err(|error| error.to_string())?;
    assert!(String::from_utf8_lossy(&prepared).contains("TacModels.ReadResult<Models.Greeting>"));
    CsharpModelStagingFixture::unknown(root.path())?;
    let source = std::fs::read(root.path().join(CsharpModelStagingFixture::SERVICE))
        .map_err(|error| error.to_string())?;
    assert!(
        ComponentScript::prepare_captured(root.path(), source, CsharpModelStagingFixture::SERVICE)
            .is_err()
    );
    Ok(())
}
