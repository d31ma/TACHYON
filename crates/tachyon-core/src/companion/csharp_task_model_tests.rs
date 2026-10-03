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
    fn collection(record: &str, leaf: &str) -> Result<(tempfile::TempDir, ModelTable), String> {
        let (root, _) = Self::models()?;
        let folder = root.path().join("client/models");
        std::fs::write(folder.join("Greeting.cs"), record).map_err(|error| error.to_string())?;
        std::fs::write(folder.join("GreetingItem.cs"), leaf).map_err(|error| error.to_string())?;
        let models = ModelTable::read_realm(root.path(), "client", "cs")?;
        Ok((root, models))
    }

    fn entries(models: &ModelTable) -> Result<String, crate::Failure> {
        CsharpTaskMembers::entries(Self::SOURCE, Self::PATH, models, &[])
    }

    fn refused_collection(record: &str, leaf: &str) -> Result<(), String> {
        let (_, models) = Self::collection(record, leaf)?;
        let failure = Self::entries(&models)
            .err()
            .ok_or_else(|| "unsupported collection accepted".to_owned())?;
        assert_eq!(failure.diagnostics()[0].spans[0].file, Self::PATH);
        assert_eq!(failure.diagnostics()[0].code.number(), 1405);
        Ok(())
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

#[test]
fn csharp_registered_task_collection_field_uses_existing_browser_codec() -> Result<(), String> {
    let (_, models) = CsharpTaskModelFixture::collection(
        "public record Greeting(System.Collections.Generic.List<GreetingItem> Items);",
        "public record GreetingItem(string Message);",
    )?;
    let entries = CsharpTaskModelFixture::entries(&models).map_err(|error| error.to_string())?;
    assert!(entries.contains("TacModels.Plain(await Instance.Read("));
    let schema = models.event_models("Greeting")?;
    assert_eq!(schema["Greeting"]["fields"]["items"], "![]GreetingItem");
    assert_eq!(schema["GreetingItem"]["fields"]["message"], "!string");
    Ok(())
}

#[test]
fn csharp_registered_task_nullable_collection_preserves_existing_optional_metadata()
-> Result<(), String> {
    let (_, models) = CsharpTaskModelFixture::collection(
        "public record Greeting(System.Collections.Generic.List<GreetingItem?>? Items);",
        "public record GreetingItem(string? Message);",
    )?;
    CsharpTaskModelFixture::entries(&models).map_err(|error| error.to_string())?;
    let schema = models.event_models("Greeting")?;
    assert_eq!(schema["Greeting"]["fields"]["items"], "?[]?GreetingItem");
    assert_eq!(schema["GreetingItem"]["fields"]["message"], "?string");
    Ok(())
}

#[test]
fn csharp_task_collection_cannot_hide_an_unregistered_leaf() -> Result<(), String> {
    CsharpTaskModelFixture::refused_collection(
        "public record Greeting(System.Collections.Generic.List<UnknownItem> Items);",
        "public record GreetingItem(string Message);",
    )
}

#[test]
fn csharp_task_collection_cannot_hide_a_recursive_record_graph() -> Result<(), String> {
    CsharpTaskModelFixture::refused_collection(
        "public record Greeting(System.Collections.Generic.List<GreetingItem> Items);",
        "public record GreetingItem(Greeting Parent);",
    )
}

#[test]
fn csharp_task_collection_cannot_hide_a_nested_collection() -> Result<(), String> {
    CsharpTaskModelFixture::refused_collection(
        "public record Greeting(System.Collections.Generic.List<System.Collections.Generic.List<GreetingItem>> Items);",
        "public record GreetingItem(string Message);",
    )
}

#[test]
fn csharp_task_collection_cannot_hide_an_arbitrary_leaf_field() -> Result<(), String> {
    CsharpTaskModelFixture::refused_collection(
        "public record Greeting(System.Collections.Generic.List<GreetingItem> Items);",
        "public record GreetingItem(object Message);",
    )
}

#[test]
fn csharp_task_collection_cannot_hide_a_map_field() -> Result<(), String> {
    CsharpTaskModelFixture::refused_collection(
        "public record Greeting(System.Collections.Generic.Dictionary<string, GreetingItem> Items);",
        "public record GreetingItem(string Message);",
    )
}
