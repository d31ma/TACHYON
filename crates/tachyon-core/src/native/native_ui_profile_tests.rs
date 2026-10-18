#![allow(clippy::expect_used)]

use super::{
    NativeBuildOptions, NativeBuildResult, NativeCompiler, NativeUiChange,
    native_ui_profile::NativeUiProfile,
};
use std::path::Path;
use tachyon_contracts::NativeTarget;

struct NativeUiProfileFixture;

impl NativeUiProfileFixture {
    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        Self::write(root.path(), "client/pages/tac.html", "<ui-panel />");
        Self::write(
            root.path(),
            "client/components/ui/panel/tac.html",
            "<main>Ready</main>",
        );
        Self::write(
            root.path(),
            "client/shared/theme.css",
            "main { color: black; }",
        );
        Self::write(
            root.path(),
            "client/manifest.json",
            r#"{"name":"UI Candidate","tac":{"application":{"id":"org.example.ui","version":"1"}}}"#,
        );
        root
    }

    fn write(root: &Path, path: &str, source: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("source");
    }

    async fn build(root: &Path, candidate: bool) -> NativeBuildResult {
        let options = NativeBuildOptions {
            output_directory: "dist".into(),
            target: NativeTarget::Macos,
            package: false,
            ..Default::default()
        };
        if candidate {
            return NativeCompiler::build_ui_candidate_async(root, &options)
                .await
                .expect("candidate metadata");
        }
        NativeCompiler::build_async(root, &options)
            .await
            .expect("ordinary metadata")
    }

    fn unchanged_records(generation: &super::NativeUiGeneration) -> Vec<serde_json::Value> {
        let document: serde_json::Value =
            serde_json::from_slice(&generation.descriptor_bytes().expect("descriptor"))
                .expect("JSON");
        document["assets"]
            .as_array()
            .expect("asset records")
            .iter()
            .filter(|asset| {
                !["tachyon-sw.js", "tachyon-sw.js.br", "tachyon-sw.js.gz"]
                    .contains(&asset["path"].as_str().expect("path"))
            })
            .cloned()
            .collect()
    }

    async fn consumer(build: &NativeBuildResult) {
        let mut command = crate::node_driver::NodeDriver::async_command();
        let shim = super::host::NativeHost::SHIM.replace("__WINDOW_CONTROLS__", "[]");
        let prefix = "globalThis.addEventListener = () => {}; globalThis.location = { hostname: 'bundle' }; let registered = 0, unregistered = 0; Object.defineProperty(globalThis, 'navigator', { value: { serviceWorker: { register() { registered++; throw new Error('native must not register a worker'); }, getRegistrations() { return Promise.resolve([{ unregister() { unregistered++; return Promise.resolve(true); } }]); } } }, configurable: true });";
        let check = "await import('./tachyon-runtime/register-sw.js'); await new Promise((resolve) => setTimeout(resolve, 0)); if (registered !== 0 || unregistered !== 1) throw new Error('wrong native registration behavior'); console.log('native cache inactive; unregistered=1; registered=0');";
        let source = format!("{prefix}\n{shim}\n{check}");
        command
            .args(["--input-type=module", "--eval", &source])
            .current_dir(build.output_directory().join("web"));
        let output = crate::node_driver::NodeDriver::run_within(
            &mut command,
            std::time::Duration::from_secs(15),
        )
        .await
        .expect("supervised actual emitted registration consumer");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "native cache inactive; unregistered=1; registered=0"
        );
    }
}

#[tokio::test]
async fn native_ui_profile_omits_only_inactive_worker_outputs_and_preserves_default_assets() {
    let ordinary_root = NativeUiProfileFixture::project();
    let candidate_root = NativeUiProfileFixture::project();
    let ordinary_build = NativeUiProfileFixture::build(ordinary_root.path(), false).await;
    let candidate_build = NativeUiProfileFixture::build(candidate_root.path(), true).await;
    let ordinary = NativeCompiler::ui_generation(&ordinary_build).expect("ordinary generation");
    let candidate = NativeCompiler::ui_generation(&candidate_build).expect("candidate generation");
    assert!(ordinary.asset("tachyon-sw.js").is_some());
    assert!(candidate.asset("tachyon-sw.js").is_none());
    assert_eq!(
        NativeUiProfileFixture::unchanged_records(&ordinary),
        NativeUiProfileFixture::unchanged_records(&candidate),
        "every non-worker asset must remain byte-identical"
    );
    for path in [
        "index.html",
        ".tachyon/register-sw.js",
        "tachyon-runtime/register-sw.js",
        "route-manifest.json",
        "tachyon-runtime/build-state.json",
        "shared/theme.css",
    ] {
        assert_eq!(ordinary.asset(path), candidate.asset(path), "{path}");
    }
    assert_eq!(
        candidate.compare(&ordinary),
        NativeUiChange::RestartRequired,
        "profile transition does not bypass non-CSS changes"
    );
    NativeUiProfileFixture::consumer(&candidate_build).await;
}

#[tokio::test]
async fn native_ui_profile_independent_static_css_builds_are_prepare_required_with_real_fingerprints()
 {
    let installed_root = NativeUiProfileFixture::project();
    let candidate_root = NativeUiProfileFixture::project();
    NativeUiProfileFixture::write(
        candidate_root.path(),
        "client/shared/theme.css",
        "main { color: blue; }",
    );
    let installed_build = NativeUiProfileFixture::build(installed_root.path(), true).await;
    let candidate_build = NativeUiProfileFixture::build(candidate_root.path(), true).await;
    let installed = NativeCompiler::ui_generation(&installed_build).expect("installed metadata");
    let candidate = NativeCompiler::ui_generation(&candidate_build).expect("candidate metadata");
    assert_ne!(
        installed.asset("shared/theme.css"),
        candidate.asset("shared/theme.css")
    );
    assert_eq!(candidate.compare(&installed), NativeUiChange::PrepareStyles);
    assert_eq!(installed.asset("index.html"), candidate.asset("index.html"));
    assert_eq!(
        installed.asset("tachyon-runtime/build-state.json"),
        candidate.asset("tachyon-runtime/build-state.json")
    );
}

#[tokio::test]
async fn native_ui_profile_retains_strict_document_and_script_changes() {
    let installed_root = NativeUiProfileFixture::project();
    let installed_build = NativeUiProfileFixture::build(installed_root.path(), true).await;
    let installed = NativeCompiler::ui_generation(&installed_build).expect("installed generation");
    let candidate_root = NativeUiProfileFixture::project();
    NativeUiProfileFixture::write(
        candidate_root.path(),
        "client/components/ui/panel/tac.html",
        "<main>Changed</main>",
    );
    let candidate_build = NativeUiProfileFixture::build(candidate_root.path(), true).await;
    let candidate = NativeCompiler::ui_generation(&candidate_build).expect("candidate generation");
    assert_eq!(
        candidate.compare(&installed),
        NativeUiChange::RestartRequired
    );
}

#[test]
fn native_ui_profile_refuses_unknown_registration_and_nonmacos_targets() {
    let root = tempfile::tempdir().expect("owned output");
    NativeUiProfileFixture::write(
        root.path(),
        ".tachyon/register-sw.js",
        "navigator.serviceWorker.register('/custom.js');",
    );
    NativeUiProfileFixture::write(root.path(), "tachyon-sw.js", "compiler-owned position");
    assert!(NativeUiProfile::prepare(true, root.path(), NativeTarget::Macos).is_err());
    assert!(root.path().join("tachyon-sw.js").exists());
    assert!(NativeUiProfile::prepare(true, root.path(), NativeTarget::Windows).is_err());
    assert!(NativeUiProfile::prepare(false, root.path(), NativeTarget::Macos).is_ok());
}
