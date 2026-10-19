#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

struct MacOsReleaseCli;

impl MacOsReleaseCli {
    const MANIFEST: &'static str = r#"{"name":"Release Check","start_url":"/","tac":{"application":{"id":"dev.tachyon.release-check","version":"1.2.0","build":3,"permissions":{"geolocation":"Shows stores near you"}},"browserApis":["geolocation"],"visibility":{"check":false}}}"#;
    const PAGE: &'static str = "<release-panel />\n";
    const COMPONENT: &'static str = "<main aria-label=\"Home\"><h1>Home</h1></main>\n";
    const SIGNING: [&'static str; 3] = [
        "TAC_APPLE_TEAM_ID",
        "TAC_APPLE_NOTARY_PROFILE",
        "TAC_MACOS_SIGNING_IDENTITY",
    ];

    fn project() -> tempfile::TempDir {
        let project = tempfile::tempdir().expect("project");
        Self::write(&project.path().join("client/manifest.json"), Self::MANIFEST);
        Self::write(&project.path().join("client/pages/tac.html"), Self::PAGE);
        Self::write(
            &project
                .path()
                .join("client/components/release/panel/tac.html"),
            Self::COMPONENT,
        );
        project
    }

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("fixture");
    }

    fn bundle(project: &Path, environment: &[(&str, &str)]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
        command
            .arg("bundle")
            .arg(project)
            .args(["--target", "macos", "--release"]);
        for name in Self::SIGNING {
            command.env_remove(name);
        }
        command
            .envs(environment.iter().copied())
            .output()
            .expect("ty starts")
    }

    fn refusal(project: &Path, environment: &[(&str, &str)]) -> String {
        let output = Self::bundle(project, environment);
        assert!(!output.status.success());
        String::from_utf8_lossy(&output.stderr).into_owned()
    }

    #[cfg(target_os = "macos")]
    fn tool(program: &str, arguments: &[&str]) -> String {
        let output = Command::new(program)
            .args(arguments)
            .output()
            .expect("tool starts");
        assert!(
            output.status.success(),
            "{program} {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        [output.stdout, output.stderr]
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .concat()
    }

    #[cfg(target_os = "macos")]
    fn built(project: &Path, environment: &[(&str, &str)]) -> String {
        let output = Self::bundle(project, environment);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    #[cfg(target_os = "macos")]
    fn check_disk_image(dmg: &Path) {
        let mount = tempfile::tempdir().expect("mount point");
        let mount_path = mount.path().to_str().expect("mount path");
        let image = dmg.to_str().expect("dmg path");
        Self::tool(
            "/usr/bin/hdiutil",
            &[
                "attach",
                "-nobrowse",
                "-readonly",
                "-mountpoint",
                mount_path,
                image,
            ],
        );
        let app = mount.path().join("ReleaseCheck.app");
        let shortcut = fs::read_link(mount.path().join("Applications"));
        let signed = Self::tool(
            "/usr/bin/codesign",
            &[
                "--verify",
                "--strict",
                "--deep",
                app.to_str().expect("app path"),
            ],
        );
        Self::tool("/usr/bin/hdiutil", &["detach", mount_path]);
        assert!(app.is_dir());
        assert_eq!(
            shortcut.expect("Applications link"),
            Path::new("/Applications")
        );
        assert!(!signed.contains("invalid"), "{signed}");
    }
}

#[test]
fn a_macos_release_names_the_missing_apple_variables_before_it_builds() {
    let project = MacOsReleaseCli::project();
    let message = MacOsReleaseCli::refusal(project.path(), &[]);
    assert!(message.contains("TY1605"), "{message}");
    assert!(
        message.contains("missing: TAC_APPLE_TEAM_ID, TAC_APPLE_NOTARY_PROFILE."),
        "{message}"
    );
    assert!(!project.path().join("dist").exists());
    let malformed = MacOsReleaseCli::refusal(
        project.path(),
        &[
            ("TAC_APPLE_TEAM_ID", "my-team"),
            ("TAC_APPLE_NOTARY_PROFILE", "tachyon-notary"),
        ],
    );
    assert!(malformed.contains("'my-team'"), "{malformed}");
    assert!(!project.path().join("dist").exists());
}

#[cfg(not(target_os = "macos"))]
#[test]
fn a_macos_release_off_a_mac_stops_before_it_signs() {
    let project = MacOsReleaseCli::project();
    let message = MacOsReleaseCli::refusal(project.path(), &[("TAC_MACOS_SIGNING_IDENTITY", "-")]);
    assert!(
        message.contains("The macOS host requires an Apple macOS build machine."),
        "{message}"
    );
    assert!(!project.path().join("dist/macos/ReleaseCheck.dmg").exists());
}

#[cfg(target_os = "macos")]
#[test]
fn an_ad_hoc_macos_release_is_a_hardened_dmg_with_the_granted_entitlements() {
    let project = MacOsReleaseCli::project();
    let printed = MacOsReleaseCli::built(project.path(), &[("TAC_MACOS_SIGNING_IDENTITY", "-")]);
    let output = project.path().join("dist/macos");
    let dmg = output.join("ReleaseCheck.dmg");
    assert!(printed.contains("ReleaseCheck.dmg"), "{printed}");
    assert!(printed.contains("signed ad hoc"), "{printed}");
    let app = output.join("ReleaseCheck.app");
    let app = app.to_str().expect("app path");
    let details = MacOsReleaseCli::tool("/usr/bin/codesign", &["--display", "--verbose=2", app]);
    assert!(details.contains("(runtime)"), "{details}");
    let entitlements = MacOsReleaseCli::tool(
        "/usr/bin/codesign",
        &["--display", "--entitlements", "-", "--xml", app],
    );
    assert!(
        entitlements.contains("com.apple.security.personal-information.location"),
        "{entitlements}"
    );
    MacOsReleaseCli::check_disk_image(&dmg);
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "needs a Developer ID Application certificate, TAC_APPLE_TEAM_ID and a notarytool profile on the Mac"]
fn a_developer_id_macos_release_is_notarized_and_stapled() {
    let team = std::env::var("TAC_APPLE_TEAM_ID").expect("TAC_APPLE_TEAM_ID");
    let profile = std::env::var("TAC_APPLE_NOTARY_PROFILE").expect("TAC_APPLE_NOTARY_PROFILE");
    let project = MacOsReleaseCli::project();
    let printed = MacOsReleaseCli::built(
        project.path(),
        &[
            ("TAC_APPLE_TEAM_ID", team.as_str()),
            ("TAC_APPLE_NOTARY_PROFILE", profile.as_str()),
        ],
    );
    assert!(!printed.contains("signed ad hoc"), "{printed}");
    let dmg = project.path().join("dist/macos/ReleaseCheck.dmg");
    let image = dmg.to_str().expect("dmg path");
    MacOsReleaseCli::tool("/usr/bin/xcrun", &["stapler", "validate", image]);
    let assessed = MacOsReleaseCli::tool(
        "/usr/sbin/spctl",
        &[
            "--assess",
            "--type",
            "open",
            "--context",
            "context:primary-signature",
            "--verbose=2",
            image,
        ],
    );
    assert!(assessed.contains("Notarized Developer ID"), "{assessed}");
    let app = project.path().join("dist/macos/ReleaseCheck.app");
    let details = MacOsReleaseCli::tool(
        "/usr/bin/codesign",
        &["--display", "--verbose=2", app.to_str().expect("app path")],
    );
    assert!(
        details.contains(&format!("TeamIdentifier={team}")),
        "{details}"
    );
    MacOsReleaseCli::check_disk_image(&dmg);
}
