#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

struct IosDeviceCli;

impl IosDeviceCli {
    const MANIFEST: &'static str = r#"{"name":"Device Check","start_url":"/","tac":{"application":{"id":"dev.tachyon.device-check","version":"1.2.0","build":3},"visibility":{"check":false}}}"#;
    const PAGE: &'static str = "<device-panel />\n";
    const COMPONENT: &'static str = "<main aria-label=\"Home\"><h1>Home</h1></main>\n";
    const SIGNING: [&'static str; 3] = [
        "TAC_APPLE_TEAM_ID",
        "TAC_IOS_PROVISIONING_PROFILE",
        "TAC_IOS_SIGNING_IDENTITY",
    ];

    fn project() -> tempfile::TempDir {
        let project = tempfile::tempdir().expect("project");
        Self::write(&project.path().join("client/manifest.json"), Self::MANIFEST);
        Self::write(&project.path().join("client/pages/tac.html"), Self::PAGE);
        Self::write(
            &project
                .path()
                .join("client/components/device/panel/tac.html"),
            Self::COMPONENT,
        );
        project
    }

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("fixture");
    }

    fn command(project: &Path, arguments: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
        command.arg("bundle").arg(project).args(arguments);
        for name in Self::SIGNING {
            command.env_remove(name);
        }
        command
    }

    fn refusal(command: &mut Command) -> String {
        let output: Output = command.output().expect("ty starts");
        assert!(!output.status.success());
        String::from_utf8_lossy(&output.stderr).into_owned()
    }
}

#[test]
fn device_is_refused_without_an_ios_app_or_when_only_staging() {
    let project = IosDeviceCli::project();
    let android = IosDeviceCli::refusal(&mut IosDeviceCli::command(
        project.path(),
        &["--target", "android", "--device"],
    ));
    assert!(android.contains("TY1001"), "{android}");
    assert!(android.contains("no iOS app is being built"), "{android}");
    let staged = IosDeviceCli::refusal(&mut IosDeviceCli::command(
        project.path(),
        &["--target", "ios", "--native", "--device"],
    ));
    assert!(staged.contains("--native only stages"), "{staged}");
    assert!(!project.path().join("dist").exists());
}

#[test]
fn a_signed_ios_build_names_the_missing_variables_before_building() {
    let project = IosDeviceCli::project();
    for flag in ["--device", "--release"] {
        let text = IosDeviceCli::refusal(&mut IosDeviceCli::command(
            project.path(),
            &["--target", "ios", flag],
        ));
        assert!(text.contains("TY1605"), "{flag}: {text}");
        assert!(
            text.contains("missing: TAC_APPLE_TEAM_ID, TAC_IOS_PROVISIONING_PROFILE."),
            "{flag}: {text}"
        );
        assert!(!project.path().join("dist/ios").exists(), "{flag}");
    }
}

#[cfg(not(target_os = "macos"))]
#[test]
fn a_signed_ios_build_off_a_mac_stops_before_signing() {
    let project = IosDeviceCli::project();
    IosDeviceCli::write(
        &project.path().join("profiles/app.mobileprovision"),
        "profile",
    );
    let mut command = IosDeviceCli::command(project.path(), &["--target", "ios", "--device"]);
    command
        .current_dir(project.path())
        .env("TAC_APPLE_TEAM_ID", "A1B2C3D4E5")
        .env(
            "TAC_IOS_PROVISIONING_PROFILE",
            "profiles/app.mobileprovision",
        );
    let text = IosDeviceCli::refusal(&mut command);
    assert!(text.contains("TY1605"), "{text}");
    assert!(
        text.contains("only an Apple macOS build machine has"),
        "{text}"
    );
}
