#![allow(clippy::expect_used)]

use super::compiler::{NativeBuildOptions, NativeCompiler};
use super::config::NativeApplication;
use std::fs;
use tachyon_contracts::NativeTarget;

struct AskingProject;

impl AskingProject {
    fn written(manifest: &serde_json::Value) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        fs::create_dir_all(root.path().join("client/pages")).expect("pages");
        fs::write(
            root.path().join("client/manifest.json"),
            serde_json::to_vec(manifest).expect("manifest"),
        )
        .expect("manifest");
        fs::write(
            root.path().join("client/pages/tac.html"),
            "<main>Home</main>",
        )
        .expect("view");
        root
    }
}

#[test]
fn the_manifest_reasons_reach_the_native_application() {
    let root = AskingProject::written(&serde_json::json!({
        "name": "Stores",
        "tac": {
            "browserApis": ["geolocation"],
            "application": { "permissions": { "geolocation": "Shows stores near you" } },
        },
    }));
    let application = NativeApplication::discover(root.path()).expect("application");
    assert_eq!(
        application.permissions.usage("geolocation"),
        Some("Shows stores near you")
    );
}

#[test]
fn a_reason_for_an_api_the_app_does_not_grant_stops_the_build() {
    let root = AskingProject::written(&serde_json::json!({
        "name": "Stores",
        "tac": { "application": { "permissions": { "geolocation": "Shows stores near you" } } },
    }));
    let error = NativeApplication::discover(root.path()).expect_err("refused");
    assert!(error.to_string().contains("TY1601"), "{error}");
    assert!(
        error.to_string().contains(
            "tac.application.permissions.geolocation explains an API the app does not use"
        ),
        "{error}"
    );
}

#[test]
fn an_ios_build_that_asks_for_location_without_a_reason_stops_before_any_tool_runs() {
    let root = AskingProject::written(&serde_json::json!({
        "name": "Stores",
        "tac": { "browserApis": ["geolocation"] },
    }));
    let options = NativeBuildOptions {
        target: NativeTarget::Ios,
        output_directory: root.path().join("dist"),
        ..NativeBuildOptions::default()
    };
    let error = NativeCompiler::build(root.path(), &options).expect_err("refused");
    let text = error.to_string();
    assert!(text.contains("TY1601"), "{text}");
    assert!(
        text.contains("add tac.application.permissions.geolocation"),
        "{text}"
    );
    assert!(!root.path().join("dist/ios").exists());
}
