use super::{CompanionLanguage, CsharpTaskMembers, owner_table};
use crate::route_types::ModelTable;

struct CsharpTaskModelFixture;

impl CsharpTaskModelFixture {
    const SOURCE: &'static str = "public class GreetingService {\n public Task<Models.Greeting> Read() { return null; }\n public string Message = \"\";\n private Task<Models.Greeting> Hidden() { return null; }\n}\n";
    const PATH: &'static str = "client/services/GreetingService.cs";

    fn models() -> Result<(tempfile::TempDir, ModelTable), String> {
        let root = tempfile::tempdir().map_err(|error| error.to_string())?;
        let folder = root.path().join("client/models");
        std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
        std::fs::write(
            folder.join("Greeting.cs"),
            "public record Greeting(string Message);",
        )
        .map_err(|error| error.to_string())?;
        let table = ModelTable::read_realm(root.path(), "client", "cs")?;
        Ok((root, table))
    }
}

#[test]
fn csharp_registered_task_model_emission_uses_existing_codec_and_native_is_unchanged()
-> Result<(), String> {
    let (_, models) = CsharpTaskModelFixture::models()?;
    let entries = CsharpTaskMembers::entries(
        CsharpTaskModelFixture::SOURCE,
        CsharpTaskModelFixture::PATH,
        &models,
        &[],
    )
    .map_err(|error| error.to_string())?;
    assert!(entries.contains("TacModels.Plain(await Instance.Read("));
    assert!(!entries.contains("Instance.Hidden("));
    let native = owner_table(
        CompanionLanguage::CSharp,
        CsharpTaskModelFixture::SOURCE,
        CsharpTaskModelFixture::PATH,
        &[],
        "",
        false,
    )
    .map_err(|error| error.to_string())?;
    assert!(!native.contains("Instance.Read("));
    assert!(native.contains("[\"message\"]"));
    Ok(())
}

#[test]
fn csharp_task_unknown_container_and_value_task_results_are_located_refusals() -> Result<(), String>
{
    let (_, models) = CsharpTaskModelFixture::models()?;
    for kind in [
        "Task<Missing>",
        "Task<List<Greeting>>",
        "Task<int>",
        "ValueTask<Greeting>",
    ] {
        let source = CsharpTaskModelFixture::SOURCE.replace("Task<Models.Greeting>", kind);
        let error = CsharpTaskMembers::entries(&source, CsharpTaskModelFixture::PATH, &models, &[]);
        assert!(error.is_err(), "{kind}");
        let message = error
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
        assert!(message.contains(CsharpTaskModelFixture::PATH), "{message}");
    }
    Ok(())
}

#[test]
fn csharp_task_model_hooks_and_provider_fields_stay_excluded() -> Result<(), String> {
    let (_, models) = CsharpTaskModelFixture::models()?;
    let entries = CsharpTaskMembers::entries(
        CsharpTaskModelFixture::SOURCE,
        CsharpTaskModelFixture::PATH,
        &models,
        &["Read".to_owned()],
    )
    .map_err(|error| error.to_string())?;
    assert!(entries.is_empty());
    Ok(())
}

#[test]
fn csharp_nested_task_model_graph_is_explicitly_pending() -> Result<(), String> {
    let (root, _) = CsharpTaskModelFixture::models()?;
    std::fs::write(
        root.path().join("client/models/Greeting.cs"),
        "public record Greeting(GreetingDetail Detail);",
    )
    .map_err(|error| error.to_string())?;
    std::fs::write(
        root.path().join("client/models/GreetingDetail.cs"),
        "public record GreetingDetail(string Message);",
    )
    .map_err(|error| error.to_string())?;
    let models = ModelTable::read_realm(root.path(), "client", "cs")?;
    assert!(
        CsharpTaskMembers::entries(
            CsharpTaskModelFixture::SOURCE,
            CsharpTaskModelFixture::PATH,
            &models,
            &[]
        )
        .is_err()
    );
    Ok(())
}
