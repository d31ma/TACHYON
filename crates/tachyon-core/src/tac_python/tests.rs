use super::runtime_assets::RuntimeAssets;
use super::{RuntimeDoctor, RuntimeInventory, RuntimeProfile};
use crate::doctor::ToolchainState;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn ordinary_projects_have_no_python_profile() {
    assert_eq!(
        RuntimeProfile::read(&json!({}))
            .unwrap_or_else(|error| panic!("ordinary manifest: {error:?}")),
        None
    );
}

#[test]
fn only_the_explicit_pinned_offline_profile_is_selected() {
    let active = json!({"tac":{"python":{"runtime":"pyodide-314.0.7","profile":"offline"}}});
    assert!(
        RuntimeProfile::read(&active)
            .unwrap_or_else(|error| panic!("pinned profile: {error:?}"))
            .is_some()
    );
    for unsupported in [
        json!({"tac":{"python":null}}),
        json!({"tac":{"python":{"runtime":"latest","profile":"offline"}}}),
        json!({"tac":{"python":{"runtime":"pyodide-314.0.7","profile":"other"}}}),
    ] {
        let failure = RuntimeProfile::read(&unsupported)
            .err()
            .unwrap_or_else(|| panic!("invalid explicit profile"));
        assert!(failure.to_string().contains("client/manifest.json"));
    }
}

#[test]
fn the_profile_cannot_relax_arbitrary_or_tampered_wasm() {
    assert_eq!(
        RuntimeProfile::asset_limit("other.wasm", b"other")
            .unwrap_or_else(|error| panic!("ordinary bound: {error:?}")),
        4 * 1024 * 1024
    );
    assert!(
        RuntimeProfile::asset_limit(
            &RuntimeAssets::output_name(RuntimeAssets::WASM),
            b"tampered"
        )
        .is_err()
    );
    assert_eq!(RuntimeProfile::SCRIPT_BYTES, 512 * 1024);
}

#[test]
fn all_five_files_include_stdlib_and_lock_dependencies() {
    let names = RuntimeAssets::names();
    assert_eq!(names.len(), 5);
    assert!(names.iter().any(|name| name.ends_with("python_stdlib.zip")));
    assert!(names.iter().any(|name| name.ends_with("pyodide-lock.json")));
}

#[test]
fn unrelated_routes_do_not_load_runtime_assets() {
    assert!(
        RuntimeInventory::reachable(&BTreeSet::new(), &BTreeMap::new(), &|_| None)
            .unwrap_or_else(|error| panic!("ordinary route: {error:?}"))
            .is_empty()
    );
    let loaded = BTreeSet::from([String::from(RuntimeInventory::ENGINE)]);
    assert!(
        RuntimeInventory::reachable(&loaded, &BTreeMap::new(), &|_| None)
            .err()
            .unwrap_or_else(|| panic!("missing reachable dependency"))
            .to_string()
            .contains("not emitted")
    );
}

#[test]
fn doctor_reports_missing_assets_without_creating_or_installing_them() {
    let folder = tempfile::tempdir().unwrap_or_else(|error| panic!("cache: {error:?}"));
    let root = folder.path().join("not-installed");
    let row = RuntimeDoctor::inspect(&root);
    assert!(matches!(row.state, ToolchainState::Incomplete(_)));
    assert!(!root.exists());
}

#[test]
fn equal_length_corruption_and_unregistered_names_fail_hash_validation() {
    let loader = RuntimeAssets::output_name("pyodide.mjs");
    let failure = RuntimeAssets::validate(&loader, &vec![0; 17_931])
        .err()
        .unwrap_or_else(|| panic!("corrupt loader"));
    assert!(failure.to_string().contains("SHA256"));
    assert!(RuntimeAssets::validate("other/pyodide.mjs", &[]).is_err());
}

#[test]
#[ignore = "requires pinned five-file core assets; set TAC_PYTHON_CORE to the verified local directory"]
fn pinned_local_inventory_qualifies_the_profile_and_all_route_dependencies() {
    let root = std::env::var_os("TAC_PYTHON_CORE")
        .unwrap_or_else(|| panic!("explicit pinned asset directory"));
    let files = RuntimeAssets::read(std::path::Path::new(&root))
        .unwrap_or_else(|error| panic!("pinned assets: {error:?}"));
    let by_path: BTreeMap<String, &[u8]> = files
        .iter()
        .map(|(name, bytes)| (name.clone(), bytes.as_slice()))
        .collect();
    let loaded = BTreeSet::from([String::from(RuntimeInventory::ENGINE)]);
    let inventory = RuntimeInventory::reachable(&loaded, &by_path, &|_| None)
        .unwrap_or_else(|error| panic!("every required runtime dependency: {error:?}"));
    assert_eq!(
        inventory.iter().map(|(_, bytes)| bytes).sum::<u64>(),
        13_531_207
    );
    let wasm = RuntimeAssets::output_name(RuntimeAssets::WASM);
    let ceiling = RuntimeProfile::asset_limit(&wasm, by_path[&wasm])
        .unwrap_or_else(|error| panic!("verified runtime Wasm: {error:?}"));
    assert_eq!(ceiling, 10 * 1024 * 1024);
    assert!(
        RuntimeDoctor::inspect(std::path::Path::new(&root))
            .state
            .is_ready()
    );
}

#[test]
fn dormant_python_assets_do_not_activate_the_profile() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error:?}"));
    std::fs::create_dir(root.path().join("client"))
        .unwrap_or_else(|error| panic!("client: {error:?}"));
    std::fs::write(
        root.path().join("client/manifest.json"),
        r#"{"tac":{"python":{"runtime":"pyodide-314.0.7","profile":"offline"}}}"#,
    )
    .unwrap_or_else(|error| panic!("profile: {error:?}"));
    let files = vec![
        (
            std::path::PathBuf::from(RuntimeInventory::ENGINE),
            Vec::new(),
        ),
        (
            std::path::PathBuf::from("index.html"),
            b"<main>ordinary</main>".to_vec(),
        ),
    ];
    assert!(
        RuntimeProfile::for_files(root.path(), &files)
            .unwrap_or_else(|error| panic!("unused profile: {error:?}"))
            .is_none()
    );
    let reached = vec![
        (
            std::path::PathBuf::from(RuntimeInventory::ENGINE),
            Vec::new(),
        ),
        (
            std::path::PathBuf::from("index.html"),
            format!("<script src='/{}'></script>", RuntimeInventory::ENGINE).into_bytes(),
        ),
    ];
    assert!(
        RuntimeProfile::for_files(root.path(), &reached)
            .unwrap_or_else(|error| panic!("reachable profile: {error:?}"))
            .is_some()
    );
}

#[test]
fn unknown_profile_options_fail_instead_of_becoming_runtime_fetch_policy() {
    let unsupported = json!({"tac":{"python":{"runtime":"pyodide-314.0.7","profile":"offline","cdn":"https://example.invalid"}}});
    assert!(
        RuntimeProfile::read(&unsupported)
            .err()
            .unwrap_or_else(|| panic!("unknown profile field"))
            .to_string()
            .contains("unknown field")
    );
}

#[test]
fn an_unused_profile_needs_no_installed_artifact_or_doctor_row() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error:?}"));
    std::fs::create_dir(root.path().join("client"))
        .unwrap_or_else(|error| panic!("client: {error:?}"));
    std::fs::write(
        root.path().join("client/manifest.json"),
        r#"{"tac":{"python":{"runtime":"pyodide-314.0.7","profile":"offline"}}}"#,
    )
    .unwrap_or_else(|error| panic!("profile: {error:?}"));
    assert!(
        RuntimeDoctor::for_project(root.path())
            .unwrap_or_else(|error| panic!("unused doctor: {error:?}"))
            .is_none()
    );
}

#[test]
fn a_profile_never_ignores_unmarked_python_service_sources() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error:?}"));
    std::fs::create_dir_all(root.path().join("client/services"))
        .unwrap_or_else(|error| panic!("services: {error:?}"));
    std::fs::write(
        root.path().join("client/manifest.json"),
        r#"{"tac":{"python":{"runtime":"pyodide-314.0.7","profile":"offline"}}}"#,
    )
    .unwrap_or_else(|error| panic!("profile: {error:?}"));
    std::fs::write(
        root.path().join("client/services/CounterService.py"),
        "class CounterService:\n    pass\n",
    )
    .unwrap_or_else(|error| panic!("invalid service: {error:?}"));
    assert!(
        RuntimeDoctor::for_project(root.path())
            .err()
            .unwrap_or_else(|| panic!("unmarked source"))
            .to_string()
            .contains("Service")
    );
}
