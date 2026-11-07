use super::CsharpTaskModels;
use crate::route_types::ModelTable;
use std::path::Path;

struct CsharpTypedExchangeFixture;

impl CsharpTypedExchangeFixture {
    const SOURCE: &'static str = "[HTTPClient({\"path\":\"/api/greeting\",\"setup\":\"GreetingSetup\"})]\npublic interface GreetingClient {\n [GetRequest]\n System.Threading.Tasks.Task<Models.Greeting> read();\n}\n[Setup]\npublic class GreetingSetup {\n public static BehaviorAdapters configure(BehaviorExecution execution) {\n  return new BehaviorAdapters();\n }\n}\n";
    const PATH: &'static str = "client/remotes/GreetingClient.cs";

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
fn csharp_registered_typed_http_exchange_adapts_bounded_json_before_model_decode()
-> Result<(), String> {
    let (_, models) = CsharpTypedExchangeFixture::models()?;
    let concrete = crate::declared_contract::DeclaredContract::concrete(
        Path::new(CsharpTypedExchangeFixture::PATH),
        CsharpTypedExchangeFixture::SOURCE,
        "tac",
    )
    .unwrap_or_else(|| CsharpTypedExchangeFixture::SOURCE.to_owned());
    let lowered = CsharpTaskModels::lower(
        Path::new(CsharpTypedExchangeFixture::PATH),
        &concrete,
        &models,
    )
    .map_err(|error| error.to_string())?
    .unwrap_or_default();
    assert!(lowered.contains("TacModels.ReadResult<Models.Greeting>"));
    assert!(lowered.contains("\"HTTP_RESPONSE\""));
    assert!(lowered.contains("!string"));
    assert!(lowered.contains("Task<Models.Greeting>"));
    assert!(!CsharpTaskModels::codec().contains("GetProperty"));
    Ok(())
}

#[test]
fn csharp_unregistered_typed_exchange_has_a_located_refusal() -> Result<(), String> {
    let (_, models) = CsharpTypedExchangeFixture::models()?;
    let source = CsharpTypedExchangeFixture::SOURCE.replace("Models.Greeting", "Models.Missing");
    let concrete = crate::declared_contract::DeclaredContract::concrete(
        Path::new(CsharpTypedExchangeFixture::PATH),
        &source,
        "tac",
    )
    .unwrap_or(source);
    let lowered = CsharpTaskModels::lower(
        Path::new(CsharpTypedExchangeFixture::PATH),
        &concrete,
        &models,
    );
    assert!(lowered.is_err());
    let message = lowered
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(message.contains(CsharpTypedExchangeFixture::PATH));
    Ok(())
}

#[test]
fn csharp_cached_task_dto_is_refused_without_changing_task_json_contract() -> Result<(), String> {
    let (_, models) = CsharpTypedExchangeFixture::models()?;
    let source = "[Service]\npublic class CachedService {\n [Cached]\n public async Task<Models.Greeting> read() { return null; }\n}\n";
    let lowered = CsharpTaskModels::lower(
        Path::new("client/services/CachedService.cs"),
        source,
        &models,
    );
    assert!(lowered.is_err());
    Ok(())
}

#[test]
fn csharp_existing_task_json_lowering_keeps_its_bytes() -> Result<(), String> {
    let (_, models) = CsharpTypedExchangeFixture::models()?;
    let source = CsharpTypedExchangeFixture::SOURCE.replace("Models.Greeting", "BehaviorJSON");
    let path = Path::new(CsharpTypedExchangeFixture::PATH);
    let concrete = crate::declared_contract::DeclaredContract::concrete(path, &source, "tac")
        .unwrap_or(source);
    let previous =
        crate::behavior::lower(path, &concrete, "tac").map_err(|error| error.to_string())?;
    let candidate =
        CsharpTaskModels::lower(path, &concrete, &models).map_err(|error| error.to_string())?;
    assert_eq!(candidate, previous);
    Ok(())
}

#[test]
fn csharp_registered_typed_collection_exchange_uses_captured_required_closed_shapes()
-> Result<(), String> {
    let (root, _) = CsharpTypedExchangeFixture::models()?;
    std::fs::write(
        root.path().join("client/models/Greeting.cs"),
        "public record Greeting(System.Collections.Generic.List<GreetingItem> Items);",
    )
    .map_err(|error| error.to_string())?;
    std::fs::write(
        root.path().join("client/models/GreetingItem.cs"),
        "public record GreetingItem(string Message);",
    )
    .map_err(|error| error.to_string())?;
    let models = ModelTable::read_realm(root.path(), "client", "cs")?;
    let path = Path::new(CsharpTypedExchangeFixture::PATH);
    let concrete = crate::declared_contract::DeclaredContract::concrete(
        path,
        CsharpTypedExchangeFixture::SOURCE,
        "tac",
    )
    .unwrap_or_else(|| CsharpTypedExchangeFixture::SOURCE.to_owned());
    let lowered = CsharpTaskModels::lower(path, &concrete, &models)
        .map_err(|error| error.to_string())?
        .unwrap_or_default();
    assert!(lowered.contains("TacModels.ReadResult<Models.Greeting>"));
    assert!(lowered.contains("![]GreetingItem"));
    assert!(lowered.contains("!string"));
    assert!(lowered.contains("GreetingItem"));
    assert!(lowered.contains("HTTP_RESPONSE"));
    Ok(())
}

#[test]
fn csharp_typed_exchange_reads_lists_arrays_plain_values_and_value_tasks() -> Result<(), String> {
    let (_, models) = CsharpTypedExchangeFixture::models()?;
    let path = Path::new(CsharpTypedExchangeFixture::PATH);
    for (kind, reading) in [
        (
            "System.Threading.Tasks.Task<List<Models.Greeting>>",
            "TacModels.ReadResultList<Models.Greeting>(__tacAnswer.ToString(), \"[]Greeting\", ",
        ),
        (
            "System.Threading.Tasks.Task<Models.Greeting[]>",
            "TacModels.ReadResultList<Models.Greeting>(__tacAnswer.ToString(), \"[]Greeting\", ",
        ),
        (
            "System.Threading.Tasks.Task<int>",
            "TacModels.ReadResultValue<int>(__tacAnswer.ToString(), \"integer\", \"{}\"); }",
        ),
        (
            "System.Threading.Tasks.Task<System.Collections.Generic.List<string>>",
            "TacModels.ReadResultList<string>(__tacAnswer.ToString(), \"[]string\", \"{}\"); }",
        ),
        (
            "System.Threading.Tasks.ValueTask<Models.Greeting>",
            "TacModels.ReadResult<Models.Greeting>(__tacAnswer.ToString(), \"Greeting\", ",
        ),
    ] {
        let source = CsharpTypedExchangeFixture::SOURCE
            .replace("System.Threading.Tasks.Task<Models.Greeting>", kind);
        let concrete = crate::declared_contract::DeclaredContract::concrete(path, &source, "tac")
            .unwrap_or(source);
        let lowered = CsharpTaskModels::lower(path, &concrete, &models)
            .map_err(|error| format!("{kind}: {error}"))?
            .unwrap_or_default();
        assert!(lowered.contains(reading), "{kind}: {lowered}");
        assert!(
            lowered.contains(&format!("public async {kind} read()")),
            "{kind}: {lowered}"
        );
        assert!(lowered.contains("\"HTTP_RESPONSE\""), "{kind}");
    }
    let array = CsharpTypedExchangeFixture::SOURCE.replace(
        "System.Threading.Tasks.Task<Models.Greeting>",
        "System.Threading.Tasks.Task<Models.Greeting[]>",
    );
    let concrete =
        crate::declared_contract::DeclaredContract::concrete(path, &array, "tac").unwrap_or(array);
    let lowered = CsharpTaskModels::lower(path, &concrete, &models)
        .map_err(|error| error.to_string())?
        .unwrap_or_default();
    assert!(lowered.contains(").ToArray(); }"), "{lowered}");
    Ok(())
}

#[test]
fn csharp_typed_exchange_refuses_maps_and_nested_lists_with_a_location() -> Result<(), String> {
    let (_, models) = CsharpTypedExchangeFixture::models()?;
    let path = Path::new(CsharpTypedExchangeFixture::PATH);
    for kind in [
        "System.Threading.Tasks.Task<Dictionary<string, Models.Greeting>>",
        "System.Threading.Tasks.Task<List<List<Models.Greeting>>>",
        "System.Threading.Tasks.ValueTask<Models.Missing>",
    ] {
        let source = CsharpTypedExchangeFixture::SOURCE
            .replace("System.Threading.Tasks.Task<Models.Greeting>", kind);
        let concrete = crate::declared_contract::DeclaredContract::concrete(path, &source, "tac")
            .unwrap_or(source);
        let message = CsharpTaskModels::lower(path, &concrete, &models)
            .err()
            .map(|error| error.to_string())
            .ok_or_else(|| format!("{kind} was accepted"))?;
        assert!(
            message.contains(CsharpTypedExchangeFixture::PATH),
            "{message}"
        );
    }
    Ok(())
}
