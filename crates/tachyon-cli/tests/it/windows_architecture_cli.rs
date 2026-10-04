#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

struct WindowsArchitectureCli;

impl WindowsArchitectureCli {
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
        let manifest = fs::read_to_string(project.join("dist/windows/artifact-manifest.json"))
            .expect("artifact manifest");
        let value: serde_json::Value = serde_json::from_str(&manifest).expect("manifest JSON");
        value["target"]["architecture"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    }
}

#[test]
fn bundle_stages_the_windows_host_for_the_chosen_architecture() {
    let project = WindowsArchitectureCli::project();
    for (choice, expected) in [
        ("arm64", "aarch64"),
        ("x64", "x86_64"),
        ("aarch64", "aarch64"),
    ] {
        let output = WindowsArchitectureCli::bundle(
            project.path(),
            &["--target", "windows", "--native", "--arch", choice],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            WindowsArchitectureCli::architecture(project.path()),
            expected
        );
    }
    let output =
        WindowsArchitectureCli::bundle(project.path(), &["--target", "windows", "--native"]);
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
    assert_eq!(
        WindowsArchitectureCli::architecture(project.path()),
        machine
    );
}

#[test]
fn an_architecture_without_a_windows_target_is_refused() {
    let project = WindowsArchitectureCli::project();
    let output = WindowsArchitectureCli::bundle(
        project.path(),
        &["--target", "linux", "--native", "--arch", "arm64"],
    );
    assert!(!output.status.success());
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(
        message.contains("--arch chooses the Windows host architecture"),
        "{message}"
    );
}
