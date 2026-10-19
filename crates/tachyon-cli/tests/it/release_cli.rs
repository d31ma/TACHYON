#![allow(clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct ReleaseCli;

impl ReleaseCli {
    const MANIFEST: &'static str = r#"{"name":"Release Check","start_url":"/","tac":{"application":{"id":"dev.tachyon.release-check","version":"1.2.0","build":3},"visibility":{"check":false}}}"#;
    const PAGE: &'static str = "<release-panel />\n";
    const COMPONENT: &'static str = "<main aria-label=\"Home\"><h1>Home</h1></main>\n";
    const SIGNING: [&'static str; 9] = [
        "TAC_ANDROID_KEYSTORE",
        "TAC_ANDROID_KEYSTORE_PASSWORD",
        "TAC_ANDROID_KEYSTORE_ALIAS",
        "TAC_ANDROID_KEYSTORE_KEY_PASSWORD",
        "TAC_WINDOWS_CERTIFICATE",
        "TAC_WINDOWS_CERTIFICATE_PASSWORD",
        "TAC_WINDOWS_CERTIFICATE_THUMBPRINT",
        "TAC_WINDOWS_CERTIFICATE_PUBLISHER",
        "TAC_WINDOWS_CERTIFICATE_TIMESTAMP_URL",
    ];
    const FAKE_GRADLE: &'static str = r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "Gradle 8.11.1"; exit 0; fi
printf '%s\n' "$*" > "$GRADLE_RECORD/arguments.txt"
printf '%s\n' "$TAC_ANDROID_KEYSTORE" > "$GRADLE_RECORD/keystore.txt"
mkdir -p app/build/outputs/apk/release app/build/outputs/bundle/release
printf 'apk' > app/build/outputs/apk/release/app-release.apk
printf 'aab' > app/build/outputs/bundle/release/app-release.aab
"#;

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

    fn command(project: &Path, arguments: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
        command.arg("bundle").arg(project).args(arguments);
        for name in Self::SIGNING {
            command.env_remove(name);
        }
        command
    }

    fn bundle(project: &Path, arguments: &[&str]) -> Output {
        Self::command(project, arguments)
            .output()
            .expect("ty starts")
    }

    fn refusal(project: &Path, arguments: &[&str]) -> String {
        let output = Self::bundle(project, arguments);
        assert!(!output.status.success());
        String::from_utf8_lossy(&output.stderr).into_owned()
    }

    #[cfg(unix)]
    fn fake_gradle(root: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt as _;
        let tools = root.join("tools");
        let gradle = tools.join("gradle");
        Self::write(&gradle, Self::FAKE_GRADLE);
        fs::set_permissions(&gradle, fs::Permissions::from_mode(0o755)).expect("executable");
        tools
    }

    #[cfg(unix)]
    fn with_fake_gradle(command: &mut Command, root: &Path) {
        let tools = Self::fake_gradle(root);
        let path = std::env::var_os("PATH").unwrap_or_default();
        let mut paths = vec![tools];
        paths.extend(std::env::split_paths(&path));
        command
            .env("PATH", std::env::join_paths(paths).expect("PATH"))
            .env("ANDROID_HOME", root)
            .env("GRADLE_RECORD", root);
    }
}

#[test]
fn release_is_refused_without_a_native_target_or_when_only_staging() {
    let project = ReleaseCli::project();
    let web = ReleaseCli::refusal(project.path(), &["--target", "web", "--release"]);
    assert!(web.contains("no native target is being built"), "{web}");
    let staged = ReleaseCli::refusal(
        project.path(),
        &["--target", "android", "--native", "--release"],
    );
    assert!(staged.contains("--native only stages"), "{staged}");
    assert!(!project.path().join("dist").exists());
}

#[test]
fn the_staged_android_project_signs_release_builds_from_the_environment() {
    let project = ReleaseCli::project();
    let output = ReleaseCli::bundle(project.path(), &["--target", "android", "--native"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let gradle = fs::read_to_string(
        project
            .path()
            .join("dist/android/ReleaseCheck/project/app/build.gradle.kts"),
    )
    .expect("gradle file");
    assert!(gradle.contains("System.getenv(\"TAC_ANDROID_KEYSTORE\")"));
    assert!(gradle.contains("signingConfig = signingConfigs.getByName(\"release\")"));
    assert!(gradle.contains("versionCode = 3\n"));
}

#[cfg(unix)]
#[test]
fn a_release_without_a_keystore_stops_before_gradle_builds() {
    let project = ReleaseCli::project();
    let tools = tempfile::tempdir().expect("tools");
    let mut command = ReleaseCli::command(project.path(), &["--target", "android", "--release"]);
    ReleaseCli::with_fake_gradle(&mut command, tools.path());
    let output = command.output().expect("ty starts");
    assert!(!output.status.success());
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains("TY1605"), "{message}");
    assert!(
        message.contains("missing: TAC_ANDROID_KEYSTORE, TAC_ANDROID_KEYSTORE_PASSWORD, TAC_ANDROID_KEYSTORE_ALIAS."),
        "{message}"
    );
    assert!(!tools.path().join("arguments.txt").exists());
}

#[cfg(unix)]
#[test]
fn a_release_publishes_the_signed_apk_and_app_bundle() {
    let project = ReleaseCli::project();
    let tools = tempfile::tempdir().expect("tools");
    ReleaseCli::write(&project.path().join("keys/upload.jks"), "keystore");
    let mut command = ReleaseCli::command(project.path(), &["--target", "android", "--release"]);
    ReleaseCli::with_fake_gradle(&mut command, tools.path());
    command
        .current_dir(project.path())
        .env("TAC_ANDROID_KEYSTORE", "keys/upload.jks")
        .env("TAC_ANDROID_KEYSTORE_PASSWORD", "store-secret")
        .env("TAC_ANDROID_KEYSTORE_ALIAS", "upload");
    let output = command.output().expect("ty starts");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let arguments = fs::read_to_string(tools.path().join("arguments.txt")).expect("arguments");
    assert_eq!(
        arguments.trim(),
        "--no-daemon --console=plain assembleRelease bundleRelease"
    );
    let keystore = fs::read_to_string(tools.path().join("keystore.txt")).expect("keystore");
    assert_eq!(
        fs::canonicalize(keystore.trim()).expect("recorded keystore"),
        fs::canonicalize(project.path().join("keys/upload.jks")).expect("keystore")
    );
    let bundle = project.path().join("dist/android/ReleaseCheck");
    assert_eq!(
        fs::read(bundle.join("ReleaseCheck.apk")).expect("apk"),
        b"apk"
    );
    assert_eq!(
        fs::read(bundle.join("ReleaseCheck.aab")).expect("aab"),
        b"aab"
    );
    let manifest = fs::read_to_string(project.path().join("dist/android/artifact-manifest.json"))
        .expect("artifact manifest");
    assert!(manifest.contains("ReleaseCheck/ReleaseCheck.aab"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ReleaseCheck.apk"), "{stdout}");
    assert!(!stdout.contains("store-secret"));
}

#[test]
fn a_windows_release_without_a_certificate_stops_before_the_host_is_compiled() {
    let project = ReleaseCli::project();
    let message = ReleaseCli::refusal(project.path(), &["--target", "windows", "--release"]);
    assert!(message.contains("TY1605"), "{message}");
    assert!(
        message.contains("TAC_WINDOWS_CERTIFICATE_PUBLISHER is missing or has no CN= part"),
        "{message}"
    );
    assert!(!message.contains("MSVC"), "{message}");
    assert!(!project.path().join("dist/windows").exists());
}

#[cfg(not(windows))]
#[test]
fn a_signed_windows_release_passes_the_signing_check_and_reaches_the_windows_tools() {
    let project = ReleaseCli::project();
    ReleaseCli::write(&project.path().join("signing.pfx"), "certificate");
    let output = ReleaseCli::command(project.path(), &["--target", "windows", "--release"])
        .current_dir(project.path())
        .env("TAC_WINDOWS_CERTIFICATE", "signing.pfx")
        .env("TAC_WINDOWS_CERTIFICATE_PASSWORD", "secret-value")
        .env("TAC_WINDOWS_CERTIFICATE_PUBLISHER", "CN=Release Check")
        .output()
        .expect("ty starts");
    assert!(!output.status.success());
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains("TY1605"), "{message}");
    assert!(!message.contains("TAC_WINDOWS_CERTIFICATE"), "{message}");
    assert!(!message.contains("secret-value"), "{message}");
}
