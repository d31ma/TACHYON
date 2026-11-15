use std::process::Command;

use serde_json::{Value, json};

use super::{ManagedStorage, fylo_url};

fn fylo_refuses_the_system_disk() -> bool {
    let directory = tempfile::tempdir().expect("probe root");
    let output = Command::new("fylo")
        .args(["exec", "--request", r#"{"op":"handshake"}"#, "--root"])
        .arg(directory.path())
        .arg("--allow-system-disk")
        .output();
    let identity = output.map_or(Value::Null, |output| {
        serde_json::from_slice(&output.stdout).unwrap_or(Value::Null)
    });
    identity["result"]["storage"]["rootLock"]
        .get("placement")
        .is_some()
}

async fn first_failure(configuration: &Value) -> String {
    let store = match ManagedStorage::connect(configuration).await {
        Ok(store) => store,
        Err(failure) => return failure.to_string(),
    };
    store
        .execute(&create_notes())
        .await
        .err()
        .map(|failure| failure.to_string())
        .unwrap_or_default()
}

fn create_notes() -> Value {
    json!({"operation": "document", "command": "createCollection", "collection": "notes"})
}

fn datasource(root: &std::path::Path, extra: &Value) -> Value {
    let mut datasource = json!({"driver": "fylo", "url": fylo_url(root)});
    for (key, value) in extra.as_object().expect("extra settings") {
        datasource[key] = value.clone();
    }
    json!({"yon": {"datasource": datasource}})
}

#[test]
fn a_development_server_lets_a_fylo_root_use_the_system_disk() {
    let configuration = json!({"yon": {"datasource": {"driver": "fylo", "url": "fylo:///data"}}});
    let allowed = ManagedStorage::with_system_disk(configuration, true);
    assert_eq!(allowed["yon"]["datasource"]["allowsystemdisk"], json!(true));
}

#[test]
fn a_production_server_keeps_the_fylo_datasource_unchanged() {
    let configuration = json!({"yon": {"datasource": {"driver": "fylo", "url": "fylo:///data"}}});
    let kept = ManagedStorage::with_system_disk(configuration.clone(), false);
    assert_eq!(kept, configuration);
}

#[test]
fn an_explicit_setting_wins_over_the_development_default() {
    let configuration =
        json!({"yon": {"datasource": {"driver": "fylo", "allowsystemdisk": false}}});
    let kept = ManagedStorage::with_system_disk(configuration.clone(), true);
    assert_eq!(kept, configuration);
}

#[test]
fn other_drivers_are_not_changed() {
    let configuration =
        json!({"yon": {"datasource": {"driver": "sqlite", "url": "sqlite::memory:"}}});
    let kept = ManagedStorage::with_system_disk(configuration.clone(), true);
    assert_eq!(kept, configuration);
}

#[test]
fn a_configuration_without_a_datasource_is_not_changed() {
    let configuration = json!({"yon": {"redis": {"url": "redis://127.0.0.1:6379"}}});
    let kept = ManagedStorage::with_system_disk(configuration.clone(), true);
    assert_eq!(kept, configuration);
}

#[tokio::test]
async fn a_root_on_the_system_disk_is_refused_without_the_setting_and_names_both_ways() {
    if !fylo_refuses_the_system_disk() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = std::fs::canonicalize(directory.path()).expect("canonical root");
    let message = first_failure(&datasource(&root, &json!({}))).await;
    assert!(message.contains("ESYSTEMDISK"), "{message}");
    assert!(message.contains("\"allowsystemdisk\": true"), "{message}");
    assert!(message.contains("FYLO_ALLOW_SYSTEM_DISK=1"), "{message}");
}

#[tokio::test]
async fn a_root_on_the_system_disk_is_used_with_the_setting() {
    if !fylo_refuses_the_system_disk() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = std::fs::canonicalize(directory.path()).expect("canonical root");
    let configuration = datasource(&root, &json!({"allowsystemdisk": true}));
    let store = ManagedStorage::connect(&configuration)
        .await
        .unwrap_or_else(|error| panic!("{error}"));
    let created = store.execute(&create_notes()).await;
    assert!(created.is_ok(), "{created:?}");
}
