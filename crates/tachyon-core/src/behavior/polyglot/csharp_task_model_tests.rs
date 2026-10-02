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
