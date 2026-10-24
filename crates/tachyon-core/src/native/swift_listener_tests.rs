#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::Path;
use std::process::Command;

struct SwiftListenerFixture;

impl SwiftListenerFixture {
    const SERVICE: &'static str =
        include_str!("../../tests/fixtures/native-swift-listeners/HeardService.swift");
    const MODEL: &'static str =
        include_str!("../../tests/fixtures/native-swift-listeners/Ping.swift");
    const CALLER: &'static str =
        include_str!("../../tests/fixtures/native-swift-listeners/HeardCaller.swift");

    fn write(root: &Path, relative: &str, source: &str) -> std::path::PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, source).expect("fixture source");
        path
    }

    fn staged(root: &Path) -> std::path::PathBuf {
        Self::write(root, "client/models/Ping.swift", Self::MODEL);
        let source = Self::write(root, "client/services/HeardService.swift", Self::SERVICE);
        let companion = NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::Swift,
            source,
            service: String::from("heard"),
            class: String::from("HeardService"),
            web_assembly: false,
            application_provider: false,
        };
        super::macos::stage_swift_companion(&[companion], root, None)
            .expect("stage")
            .expect("a Swift companion")
    }

    fn available() -> bool {
        Command::new("swiftc")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn run(root: &Path, staged: &Path) -> String {
        let json = Self::write(
            root,
            "apple_json.swift",
            super::macos::MacOsHostGenerator::APPLE_JSON_REQUEST,
        );
        let caller = Self::write(root, "caller.swift", Self::CALLER);
        let program = root.join("caller");
        let compiled = Command::new("swiftc")
            .arg("-parse-as-library")
            .arg(staged)
            .arg(&json)
            .arg(&caller)
            .arg("-o")
            .arg(&program)
            .output()
            .expect("swiftc");
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let output = Command::new(&program).output().expect("the caller runs");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

#[test]
fn a_linked_swift_async_listener_finishes_before_the_next_listener() {
    if !SwiftListenerFixture::available() {
        eprintln!("skipping: swiftc is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let staged = SwiftListenerFixture::staged(root.path());
    let printed = SwiftListenerFixture::run(root.path(), &staged);
    for wanted in [
        "\"task\":true",
        "first send {\"value\":null}",
        "first heard {\"value\":\"a1s1\"}",
        "second send {\"value\":null}",
        "second heard {\"value\":\"a1s1a2s2\"}",
        "dispose {\"value\":null}",
    ] {
        assert!(printed.contains(wanted), "missing {wanted} in {printed}");
    }
}
