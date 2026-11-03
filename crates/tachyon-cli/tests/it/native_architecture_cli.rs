#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

struct NativeArchitectureCli;

impl NativeArchitectureCli {
    const MANIFEST: &'static str = r#"{"name":"ArchCheck","start_url":"/","tac":{"application":{"id":"dev.tachyon.arch-check","version":"0.0.1"},"visibility":{"check":false}}}"#;
    const PAGE: &'static str = "<arch-panel />\n";
    const COMPONENT: &'static str = "<main aria-label=\"Home\"><h1>Home</h1></main>\n";

    fn project() -> tempfile::TempDir {
        let project = tempfile::tempdir().expect("project");
        Self::write(&project.path().join("client/manifest.json"), Self::MANIFEST);
        Self::write(&project.path().join("client/pages/tac.html"), Self::PAGE);
        Self::write(
            &project.path().join("client/components/arch/panel/tac.html"),
            Self::COMPONENT,
        );
        project
    }

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("fixture");
    }

    fn bundle(project: &Path, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ty"))
            .arg("bundle")
            .arg(project)
            .args(arguments)
            .output()
            .expect("ty starts")
    }

    fn architecture(project: &Path) -> String {
        Self::target_architecture(project, "windows")
    }

    fn target_architecture(project: &Path, target: &str) -> String {
        let path = project
            .join("dist")
            .join(target)
            .join("artifact-manifest.json");
        let manifest = fs::read_to_string(path).expect("artifact manifest");
        let value: serde_json::Value = serde_json::from_str(&manifest).expect("manifest JSON");
        value["target"]["architecture"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    }
}

#[test]
fn bundle_stages_the_windows_host_for_the_chosen_architecture() {
    let project = NativeArchitectureCli::project();
    for (choice, expected) in [
        ("arm64", "aarch64"),
        ("x64", "x86_64"),
        ("aarch64", "aarch64"),
    ] {
        let output = NativeArchitectureCli::bundle(
            project.path(),
            &["--target", "windows", "--native", "--arch", choice],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            NativeArchitectureCli::architecture(project.path()),
            expected
        );
    }
    let output =
        NativeArchitectureCli::bundle(project.path(), &["--target", "windows", "--native"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let machine = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x86_64"
    };
    assert_eq!(NativeArchitectureCli::architecture(project.path()), machine);
}

#[test]
fn bundle_stages_the_macos_host_for_the_chosen_architecture() {
    let project = NativeArchitectureCli::project();
    for (choice, expected) in [
        ("universal", "universal"),
        ("x64", "x86_64"),
        ("arm64", "aarch64"),
    ] {
        let output = NativeArchitectureCli::bundle(
            project.path(),
            &["--target", "macos", "--native", "--arch", choice],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            NativeArchitectureCli::target_architecture(project.path(), "macos"),
            expected
        );
    }
}

#[test]
fn an_architecture_that_no_built_target_honors_is_refused() {
    let project = NativeArchitectureCli::project();
    for target in ["linux", "windows"] {
        let output = NativeArchitectureCli::bundle(
            project.path(),
            &["--target", target, "--native", "--arch", "universal"],
        );
        assert!(!output.status.success(), "{target}");
        let message = String::from_utf8_lossy(&output.stderr);
        assert!(
            message.contains("--arch universal cannot be honored by any target being built"),
            "{target}: {message}"
        );
        assert!(message.contains("TY1001"), "{target}: {message}");
    }
    let output =
        NativeArchitectureCli::bundle(project.path(), &["--target", "web", "--arch", "x64"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--arch x64 cannot be honored"));
}

#[test]
fn targets_that_do_not_take_the_architecture_say_so_in_a_note() {
    let project = NativeArchitectureCli::project();
    let output = NativeArchitectureCli::bundle(
        project.path(),
        &[
            "--target",
            "windows,android,macos",
            "--skip-package",
            "--arch",
            "universal",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(!printed.contains("Android ABIs"), "{printed}");
    assert!(
        printed.contains("Note: --arch universal is for macOS and Android only"),
        "{printed}"
    );
    for target in ["macos", "android"] {
        assert_eq!(
            NativeArchitectureCli::target_architecture(project.path(), target),
            "universal"
        );
    }
}

#[test]
fn an_android_build_takes_one_abi_or_both() {
    let project = NativeArchitectureCli::project();
    for (choice, expected) in [
        ("arm64", "aarch64"),
        ("x64", "x86_64"),
        ("universal", "universal"),
    ] {
        let output = NativeArchitectureCli::bundle(
            project.path(),
            &["--target", "android", "--native", "--arch", choice],
        );
        assert!(
            output.status.success(),
            "{choice}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            NativeArchitectureCli::target_architecture(project.path(), "android"),
            expected
        );
        let gradle = fs::read_to_string(
            project
                .path()
                .join("dist/android/ArchCheck/project/app/build.gradle.kts"),
        )
        .expect("generated Gradle file");
        assert!(gradle.contains("abiFilters += listOf("), "{gradle}");
    }
}
