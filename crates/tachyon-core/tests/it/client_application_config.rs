#![allow(clippy::expect_used)]

use std::fs;
use tachyon_core::{BuildOptions, ProjectDiscovery, WebCompiler};

fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("project");
    fs::create_dir_all(root.path().join("client/pages")).expect("pages");
    fs::write(root.path().join("client/pages/tac.html"), "<test-view1 />").expect("page");
    fs::create_dir_all(root.path().join("client/components/test/view1")).expect("component");
    fs::write(
        root.path().join("client/components/test/view1/tac.html"),
        "<h1>Public</h1>",
    )
    .expect("view");
    fs::create_dir_all(root.path().join("server")).expect("server");
    root
}

#[test]
fn bundle_publishes_the_manifest_tac_member_without_opening_secrets() {
    let root = project();
    fs::write(
        root.path().join("client/manifest.json"),
        r#"{"name":"Public","start_url":"/","tac":{"api":{"url":"/api"}}}"#,
    )
    .expect("manifest");
    fs::write(
        root.path().join("server/application.json"),
        r#"{"datasource":{"host":"private-host"}}"#,
    )
    .expect("configuration");
    fs::create_dir(root.path().join(".env")).expect("unreadable secret shape");
    let project = ProjectDiscovery::discover(root.path()).expect("captured project");
    fs::write(
        root.path().join("client/manifest.json"),
        r#"{"name":"Public","tac":{"api":{"url":"/changed"}}}"#,
    )
    .expect("later edit");
    let output = root.path().join("dist/web");
    let options = BuildOptions {
        output_directory: output.clone(),
        ..BuildOptions::default()
    };
    WebCompiler::build_project(&project, &options).expect("bundle");
    let public =
        fs::read_to_string(output.join("manifest.webmanifest")).expect("published manifest");
    let manifest: serde_json::Value = serde_json::from_str(&public).expect("JSON");
    assert_eq!(manifest["tac"], serde_json::json!({"api":{"url":"/api"}}));
    assert!(!public.contains("private-host"));
    assert!(!output.join(".env").exists());
    assert!(!output.join("server/application.json").exists());
    assert!(!output.join(".tachyon/application.json").exists());
    WebCompiler::build(root.path(), &options).expect("fresh build");
    let fresh = fs::read_to_string(output.join("manifest.webmanifest")).expect("fresh manifest");
    assert!(fresh.contains("/changed"));
}

#[test]
fn a_tac_section_left_in_application_json_names_the_manifest() {
    let root = project();
    fs::write(
        root.path().join("server/application.json"),
        r#"{"tac":{"api":{"url":"/api"}},"yon":{}}"#,
    )
    .expect("configuration");
    let error = ProjectDiscovery::discover(root.path()).expect_err("tac in application.json");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("manifest.json")),
        "{error:?}"
    );
}

#[test]
fn configuration_at_the_project_root_names_where_it_moved() {
    for (name, destination) in [
        ("manifest.json", "client/manifest.json"),
        ("application.json", "server/application.json"),
        (
            "application-production.json",
            "server/application-<profile>.json",
        ),
    ] {
        let root = project();
        fs::write(root.path().join(name), r#"{"name":"Catalog"}"#).expect("legacy file");
        let error = ProjectDiscovery::discover(root.path()).expect_err(name);
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.message.contains(destination)),
            "{name}: {error:?}"
        );
    }
}

#[test]
fn the_manifest_tac_member_is_held_to_the_public_rules() {
    let root = project();
    fs::write(
        root.path().join("client/manifest.json"),
        r#"{"name":"Public","tac":{"api":{"password":"hunter2"}}}"#,
    )
    .expect("manifest");
    assert!(ProjectDiscovery::discover(root.path()).is_err());
}
