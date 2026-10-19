#![allow(clippy::expect_used)]

use super::{NativeBuildOptions, NativeBuildResult, NativeCompiler, native_target_directory};
use std::fs;
use std::path::Path;
use tachyon_contracts::NativeTarget;

struct NativeReproducibleBuildTests {
    workspace: tempfile::TempDir,
}

impl NativeReproducibleBuildTests {
    const MANIFEST: &'static str = r#"{"name":"Repeat Build","start_url":"/","tac":{"application":{"id":"dev.tachyon.repeat-build","version":"1.2.0"}}}"#;
    const PAGES: [(&'static str, &'static str); 5] = [
        ("client/pages/tac.html", "<repeat-home />"),
        ("client/pages/help/tac.html", "<repeat-help />"),
        (
            "client/components/repeat/home/tac.html",
            "<main><h1>Home</h1><a href=\"/help\">Help</a></main>",
        ),
        (
            "client/components/repeat/help/tac.html",
            "<main><h1>Help</h1></main>",
        ),
        ("client/manifest.json", Self::MANIFEST),
    ];

    fn new() -> Self {
        let fixture = Self {
            workspace: tempfile::tempdir().expect("workspace"),
        };
        for (relative, contents) in Self::PAGES {
            let path = fixture.project().join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            fs::write(path, contents).expect("source");
        }
        fixture
    }

    fn project(&self) -> std::path::PathBuf {
        self.workspace.path().join("project")
    }

    async fn build(
        &self,
        output: &str,
        target: NativeTarget,
        package: bool,
        incremental: bool,
    ) -> (NativeBuildResult, Vec<u8>) {
        let output = self.workspace.path().join(output);
        let options = NativeBuildOptions {
            output_directory: output.clone(),
            target,
            package,
            incremental,
            ..NativeBuildOptions::default()
        };
        let result = NativeCompiler::build_async(self.project(), &options)
            .await
            .unwrap_or_else(|failure| panic!("{failure}"));
        let manifest = Self::artifact_manifest(&output, target);
        (result, manifest)
    }

    fn artifact_manifest(output: &Path, target: NativeTarget) -> Vec<u8> {
        fs::read(
            output
                .join(native_target_directory(target))
                .join("artifact-manifest.json"),
        )
        .expect("artifact manifest")
    }

    async fn assert_repeatable(target: NativeTarget, package: bool) {
        let fixture = Self::new();
        let (first, first_manifest) = fixture.build("first", target, package, false).await;
        let (_, second_manifest) = fixture.build("second", target, package, false).await;
        assert_eq!(
            String::from_utf8_lossy(&first_manifest),
            String::from_utf8_lossy(&second_manifest),
            "two fresh {target:?} builds differ"
        );
        assert_eq!(first.reused_routes(), 0);
        let (reused, reused_manifest) = fixture.build("first", target, package, true).await;
        assert_eq!(reused.reused_routes(), first.route_count());
        assert_eq!(reused.compiled_routes(), 0);
        assert_eq!(
            String::from_utf8_lossy(&reused_manifest),
            String::from_utf8_lossy(&first_manifest),
            "an incremental {target:?} build differs from a fresh one"
        );
    }
}

#[tokio::test]
async fn staged_native_apps_are_byte_identical_when_built_twice() {
    for target in [
        NativeTarget::Linux,
        NativeTarget::Macos,
        NativeTarget::Windows,
        NativeTarget::Android,
        NativeTarget::Ios,
    ] {
        NativeReproducibleBuildTests::assert_repeatable(target, false).await;
    }
}

#[tokio::test]
async fn a_native_build_without_incremental_reuse_compiles_every_route_again() {
    let fixture = NativeReproducibleBuildTests::new();
    let target = NativeTarget::Linux;
    let (first, _) = fixture.build("dist", target, false, true).await;
    assert_eq!(first.compiled_routes(), first.route_count());
    let (forced, _) = fixture.build("dist", target, false, false).await;
    assert_eq!(forced.reused_routes(), 0);
    assert_eq!(forced.compiled_routes(), first.route_count());
}

#[tokio::test]
#[ignore = "Requires GTK 4 and WebKitGTK 6.0 development files; run explicitly on the Linux lane"]
async fn a_packaged_linux_app_is_byte_identical_when_built_twice() {
    NativeReproducibleBuildTests::assert_repeatable(NativeTarget::Linux, true).await;
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "Requires Xcode; run explicitly on the macOS lane"]
async fn a_packaged_macos_app_is_byte_identical_when_built_twice() {
    NativeReproducibleBuildTests::assert_repeatable(NativeTarget::Macos, true).await;
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "Requires Xcode and the iOS simulator SDK; run explicitly on the macOS lane"]
async fn a_packaged_ios_app_is_byte_identical_when_built_twice() {
    NativeReproducibleBuildTests::assert_repeatable(NativeTarget::Ios, true).await;
}

#[cfg(target_os = "windows")]
#[tokio::test]
#[ignore = "Requires MSVC and TAC_WEBVIEW2_SDK in a developer prompt; run explicitly on the Windows lane"]
async fn a_packaged_windows_app_is_byte_identical_when_built_twice() {
    NativeReproducibleBuildTests::assert_repeatable(NativeTarget::Windows, true).await;
}

#[tokio::test]
#[ignore = "Requires ANDROID_HOME, Gradle and network access to Google Maven; run explicitly on the Android lane"]
async fn a_packaged_android_app_is_byte_identical_when_built_twice() {
    NativeReproducibleBuildTests::assert_repeatable(NativeTarget::Android, true).await;
}
