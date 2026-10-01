use super::{RuntimeAssets, RuntimeDoctor, RuntimeInventory, RuntimeProfile};
use crate::doctor::ToolchainState;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn ordinary_projects_have_no_python_profile() {
    assert_eq!(
        RuntimeProfile::read(&json!({})).expect("ordinary manifest"),
        None
    );
}

#[test]
fn only_the_explicit_pinned_offline_profile_is_selected() {
    let active = json!({"tac":{"python":{"runtime":"pyodide-314.0.7","profile":"offline"}}});
    assert!(
        RuntimeProfile::read(&active)
            .expect("pinned profile")
            .is_some()
    );
    for unsupported in [
        json!({"tac":{"python":null}}),
        json!({"tac":{"python":{"runtime":"latest","profile":"offline"}}}),
        json!({"tac":{"python":{"runtime":"pyodide-314.0.7","profile":"other"}}}),
    ] {
        let failure = RuntimeProfile::read(&unsupported).expect_err("invalid explicit profile");
        assert!(failure.to_string().contains("client/manifest.json"));
    }
}

#[test]
fn the_profile_cannot_relax_arbitrary_or_tampered_wasm() {
    let profile = RuntimeProfile;
    assert_eq!(
        profile
            .asset_limit("other.wasm", b"other")
            .expect("ordinary bound"),
        4 * 1024 * 1024
    );
    assert!(
        profile
            .asset_limit(
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
            .expect("ordinary route")
            .is_empty()
    );
    let loaded = BTreeSet::from([String::from(RuntimeInventory::ENGINE)]);
    assert!(
        RuntimeInventory::reachable(&loaded, &BTreeMap::new(), &|_| None)
            .expect_err("missing reachable dependency")
            .to_string()
            .contains("not emitted")
    );
}

#[test]
fn doctor_reports_missing_assets_without_creating_or_installing_them() {
    let folder = tempfile::tempdir().expect("cache");
    let root = folder.path().join("not-installed");
    let row = RuntimeDoctor::inspect(&root);
    assert!(matches!(row.state, ToolchainState::Incomplete(_)));
    assert!(!root.exists());
}

#[test]
fn equal_length_corruption_and_unregistered_names_fail_hash_validation() {
    let loader = RuntimeAssets::output_name("pyodide.mjs");
    let failure = RuntimeAssets::validate(&loader, &vec![0; 17_931]).expect_err("corrupt loader");
    assert!(failure.to_string().contains("SHA256"));
    assert!(RuntimeAssets::validate("other/pyodide.mjs", &[]).is_err());
}

#[test]
#[ignore = "requires pinned five-file core assets; set TAC_PYTHON_CORE to the verified local directory"]
fn pinned_local_inventory_qualifies_the_profile_and_all_route_dependencies() {
    let root = std::env::var_os("TAC_PYTHON_CORE").expect("explicit pinned asset directory");
    let files = RuntimeAssets::read(std::path::Path::new(&root)).expect("pinned assets");
    let by_path: BTreeMap<String, &[u8]> = files
        .iter()
        .map(|(name, bytes)| (name.clone(), bytes.as_slice()))
        .collect();
    let loaded = BTreeSet::from([String::from(RuntimeInventory::ENGINE)]);
    let inventory = RuntimeInventory::reachable(&loaded, &by_path, &|_| None)
        .expect("every required runtime dependency");
    assert_eq!(
        inventory.iter().map(|(_, bytes)| bytes).sum::<u64>(),
        13_531_207
    );
    let wasm = RuntimeAssets::output_name(RuntimeAssets::WASM);
    let ceiling = RuntimeProfile
        .asset_limit(&wasm, by_path[&wasm])
        .expect("verified runtime Wasm");
    assert_eq!(ceiling, 10 * 1024 * 1024);
    assert!(
        RuntimeDoctor::inspect(std::path::Path::new(&root))
            .state
            .is_ready()
    );
}
