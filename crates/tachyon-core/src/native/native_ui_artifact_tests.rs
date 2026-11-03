#![allow(clippy::expect_used)]

use super::{
    NativeBuildOptions, NativeBuildResult, NativeCompiler, NativeUiChange, NativeUiGeneration,
};
use cap_std::{ambient_authority, fs::Dir};
use std::path::Path;
use tachyon_contracts::NativeTarget;

struct NativeUiBuildFixture;

impl NativeUiBuildFixture {
    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        Self::write(root.path(), "client/pages/tac.html", "<live-panel />");
        Self::write(
            root.path(),
            "client/components/live/panel/tac.html",
            "<main>{counter.count}</main>",
        );
        Self::write(
            root.path(),
            "client/services/CounterService.swift",
            "@Service\nfinal class CounterService { var count: Int = 0 }\n",
        );
        Self::write(
            root.path(),
            "client/shared/theme.css",
            "main { color: black; }",
        );
        Self::write(
            root.path(),
            "client/manifest.json",
            r#"{"name":"Live Updates","start_url":"/","tac":{"application":{"id":"org.example.live","version":"1"}}}"#,
        );
        root
    }

    fn write(root: &Path, path: &str, source: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("source");
    }

    async fn build(root: &Path) -> NativeBuildResult {
        let options = NativeBuildOptions {
            output_directory: "dist".into(),
            target: NativeTarget::Macos,
            package: false,
            ..Default::default()
        };
        NativeCompiler::build_async(root, &options)
            .await
            .expect("real generated native metadata")
    }

    fn document(generation: &NativeUiGeneration) -> serde_json::Value {
        serde_json::from_slice(&generation.descriptor_bytes().expect("descriptor"))
            .expect("descriptor JSON")
    }
}

#[tokio::test]
async fn native_ui_artifact_emits_a_descriptor_from_generated_host_companion_routes_and_assets() {
    let root = NativeUiBuildFixture::project();
    let build = NativeUiBuildFixture::build(root.path()).await;
    let generation = NativeCompiler::ui_generation(&build).expect("actual artifact fingerprint");
    let document = NativeUiBuildFixture::document(&generation);
    assert_eq!(document["contract"]["applicationId"], "org.example.live");
    assert!(generation.asset("index.html").is_some());
    assert_eq!(
        generation.asset("shared/theme.css"),
        Some(b"main { color: black; }".as_slice())
    );
    assert!(generation.asset("tachyon-runtime/tac-client.js").is_some());
    assert!(
        build
            .output_directory()
            .join("project/TachyonCompanion.swift")
            .is_file()
    );
    let destination = tempfile::tempdir().expect("explicit owned descriptor output");
    let output =
        Dir::open_ambient_dir(destination.path(), ambient_authority()).expect("output authority");
    let name = NativeCompiler::emit_ui_descriptor(&build, &output).expect("opt-in emission");
    assert_eq!(
        std::fs::read(destination.path().join(&name)).expect("emitted descriptor"),
        generation.descriptor_bytes().expect("descriptor")
    );
    assert!(NativeCompiler::emit_ui_descriptor(&build, &output).is_err());
    assert!(
        NativeCompiler::ui_generation(&build).is_ok(),
        "emission did not mutate original build"
    );
    assert!(
        !build.output_directory().join(name).exists(),
        "default artifacts unchanged"
    );
}

#[tokio::test]
async fn native_ui_artifact_keeps_native_identity_stable_but_detects_generated_browser_cache_changes()
 {
    let installed = NativeUiBuildFixture::project();
    let candidate = NativeUiBuildFixture::project();
    NativeUiBuildFixture::write(
        candidate.path(),
        "client/shared/theme.css",
        "main { color: blue; }",
    );
    let installed_build = NativeUiBuildFixture::build(installed.path()).await;
    let candidate_build = NativeUiBuildFixture::build(candidate.path()).await;
    let installed = NativeCompiler::ui_generation(&installed_build).expect("installed metadata");
    let candidate = NativeCompiler::ui_generation(&candidate_build).expect("candidate metadata");
    assert_eq!(
        NativeUiBuildFixture::document(&installed)["contract"],
        NativeUiBuildFixture::document(&candidate)["contract"],
        "UI bytes and absolute staging paths must not alter native identity"
    );
    assert_ne!(
        installed.asset("shared/theme.css"),
        candidate.asset("shared/theme.css")
    );
    assert_ne!(
        installed.asset("tachyon-sw.js"),
        candidate.asset("tachyon-sw.js")
    );
    assert_eq!(
        candidate.compare(&installed),
        NativeUiChange::RestartRequired,
        "default browser-cache byte changes remain a strict blocker before the opt-in native candidate profile"
    );
}

#[tokio::test]
async fn native_ui_artifact_fingerprints_actual_grants_and_companion_code() {
    let installed_root = NativeUiBuildFixture::project();
    let installed_build = NativeUiBuildFixture::build(installed_root.path()).await;
    let installed = NativeCompiler::ui_generation(&installed_build).expect("installed generation");
    for change in ["grant", "companion"] {
        let candidate_root = NativeUiBuildFixture::project();
        if change == "grant" {
            NativeUiBuildFixture::write(
                candidate_root.path(),
                "client/manifest.json",
                r#"{"name":"Live Updates","start_url":"/","tac":{"nativeCapabilities":["filePicker"],"application":{"id":"org.example.live","version":"1"}}}"#,
            );
        } else {
            NativeUiBuildFixture::write(
                candidate_root.path(),
                "client/services/CounterService.swift",
                "@Service\nfinal class CounterService { var count: Int = 1 }\n",
            );
        }
        let candidate_build = NativeUiBuildFixture::build(candidate_root.path()).await;
        let candidate =
            NativeCompiler::ui_generation(&candidate_build).expect("candidate generation");
        assert_ne!(
            NativeUiBuildFixture::document(&installed)["contract"],
            NativeUiBuildFixture::document(&candidate)["contract"],
            "{change}"
        );
        assert_eq!(
            candidate.compare(&installed),
            NativeUiChange::RestartRequired
        );
    }
}

#[tokio::test]
async fn native_ui_artifact_refuses_mutation_after_the_build_receipt() {
    for changed in [
        "project/TachyonCompanion.swift",
        "tachyon.host.json",
        "native-index.json",
        "artifact-manifest.json",
        "web/index.html",
    ] {
        let root = NativeUiBuildFixture::project();
        let build = NativeUiBuildFixture::build(root.path()).await;
        std::fs::write(build.output_directory().join(changed), "unrecorded change")
            .expect("owned fixture mutation");
        assert!(NativeCompiler::ui_generation(&build).is_err(), "{changed}");
    }
}
