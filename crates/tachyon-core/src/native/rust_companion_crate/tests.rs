use super::RustCompanionCrate;
use serde_json::json;
use std::path::{Path, PathBuf};

struct RustCompanionCrateTests;

impl RustCompanionCrateTests {
    fn declaration(value: serde_json::Value) -> RustCompanionCrate {
        serde_json::from_value(value).unwrap_or_else(|error| panic!("declaration: {error}"))
    }

    fn swarx() -> RustCompanionCrate {
        Self::declaration(json!({
            "dependencies": {
                "serde_json": "=1.0.151",
                "serde": {"version": "=1.0.229", "features": ["derive"]}
            },
            "platformDependencies": {
                "windows": {"windows-sys": {"version": "=0.61.2", "features": ["Win32_Foundation", "Win32_System_Threading"]}},
                "linux": {"libc": {"version": "=0.2.189", "defaultFeatures": false}}
            },
            "executables": {"observatory-event": "/shared/native/EventEmitterMain.rs"}
        }))
    }

    fn problem(value: serde_json::Value) -> String {
        Self::declaration(value).problem().unwrap_or_default()
    }
}

#[test]
fn a_declaration_with_platform_crates_and_an_executable_renders_one_offline_cargo_manifest() {
    let declaration = RustCompanionCrateTests::swarx();
    assert_eq!(declaration.problem(), None);
    assert!(declaration.uses_cargo() && declaration.has_dependencies());
    let binaries = vec![(
        String::from("observatory-event"),
        PathBuf::from("/stage/project/rust-companion/bin/observatory-event.rs"),
    )];
    let manifest = declaration.manifest(
        Path::new("/stage/project/companion.rs"),
        &["cdylib", "rlib"],
        &binaries,
    );
    for expected in [
        "[package]\nname = \"tachyon-companion\"",
        "[workspace]\n",
        "[lib]\nname = \"tachyon_companion\"\npath = \"/stage/project/companion.rs\"\ncrate-type = [\"cdylib\", \"rlib\"]\n",
        "[[bin]]\nname = \"observatory-event\"\npath = \"/stage/project/rust-companion/bin/observatory-event.rs\"\n",
        "[dependencies]\nserde = { version = \"=1.0.229\", features = [\"derive\"] }\nserde_json = { version = \"=1.0.151\" }\n",
        "[target.'cfg(target_os = \"linux\")'.dependencies]\nlibc = { version = \"=0.2.189\", default-features = false }\n",
        "[target.'cfg(windows)'.dependencies]\nwindows-sys = { version = \"=0.61.2\", features = [\"Win32_Foundation\", \"Win32_System_Threading\"] }\n",
    ] {
        assert!(manifest.contains(expected), "{expected}\n---\n{manifest}");
    }
    assert!(!manifest.contains("git") && !manifest.contains("registry"));
}

#[test]
fn windows_paths_are_escaped_inside_the_cargo_manifest() {
    let manifest = RustCompanionCrate::default().manifest(
        Path::new(r"C:\Users\ayode\stage\project\companion.rs"),
        &["cdylib", "rlib"],
        &[],
    );
    assert!(
        manifest.contains(r#"path = "C:\\Users\\ayode\\stage\\project\\companion.rs""#),
        "{manifest}"
    );
}

#[test]
fn an_empty_declaration_keeps_the_single_file_rustc_build() {
    let declaration = RustCompanionCrate::default();
    assert!(!declaration.uses_cargo());
    assert_eq!(declaration.problem(), None);
    let executables_only = RustCompanionCrateTests::declaration(
        json!({"executables": {"probe": "/shared/native/ProbeMain.rs"}}),
    );
    assert!(executables_only.uses_cargo() && !executables_only.has_dependencies());
}

#[test]
fn crate_versions_must_be_exact_so_the_vendored_copy_matches() {
    for version in ["1.0.151", "^1.0", "=1.0", "=1.0.x", "*"] {
        let problem =
            RustCompanionCrateTests::problem(json!({"dependencies": {"serde_json": version}}));
        assert!(problem.contains("not exact"), "{version}: {problem}");
    }
    let feature = RustCompanionCrateTests::problem(
        json!({"dependencies": {"serde": {"version": "=1.0.229", "features": ["derive\"]"]}}}),
    );
    assert!(feature.contains("not a Cargo feature name"), "{feature}");
    let name = RustCompanionCrateTests::problem(json!({"dependencies": {"../evil": "=1.0.0"}}));
    assert!(name.contains("not a crate name"), "{name}");
}

#[test]
fn platform_crates_name_a_platform_that_links_rust() {
    let problem = RustCompanionCrateTests::problem(
        json!({"platformDependencies": {"freebsd": {"libc": "=0.2.189"}}}),
    );
    assert!(
        problem.contains("use android, ios, linux, macos or windows"),
        "{problem}"
    );
    let ios = RustCompanionCrateTests::declaration(
        json!({"platformDependencies": {"ios": {"objc2": "=0.6.3"}}}),
    );
    assert_eq!(ios.problem(), None);
    let manifest = ios.manifest(Path::new("/stage/companion.rs"), &["staticlib"], &[]);
    assert!(
        manifest.contains("[target.'cfg(target_os = \"ios\")'.dependencies]\nobjc2 = "),
        "{manifest}"
    );
    let nested = RustCompanionCrateTests::problem(
        json!({"platformDependencies": {"windows": {"windows-sys": "0.61"}}}),
    );
    assert!(
        nested.contains("platformDependencies.windows.windows-sys"),
        "{nested}"
    );
}

#[test]
fn executables_need_a_plain_name_and_a_client_rust_class_file() {
    for name in [
        "Observatory",
        "observatory_event",
        "9event",
        "tachyon-companion",
        "",
    ] {
        let problem = RustCompanionCrateTests::problem(
            json!({"executables": {name: "/shared/native/EventEmitterMain.rs"}}),
        );
        assert!(problem.contains("executable name"), "{name}: {problem}");
    }
    for entry in [
        "shared/native/EventEmitterMain.rs",
        "/shared/../services/EventEmitterMain.rs",
        "/shared/native/eventEmitterMain.rs",
        "/shared/native/EventEmitterMain.js",
        r"/shared\native\EventEmitterMain.rs",
    ] {
        let problem = RustCompanionCrateTests::problem(json!({"executables": {"event": entry}}));
        assert!(
            problem.contains("pub fn run() -> i32"),
            "{entry}: {problem}"
        );
    }
    assert_eq!(
        RustCompanionCrate::entry_class("/shared/native/EventEmitterMain.rs"),
        Some("EventEmitterMain")
    );
}

#[test]
fn unknown_fields_and_dependency_shapes_are_refused() {
    assert!(serde_json::from_value::<RustCompanionCrate>(json!({"targets": {}})).is_err());
    assert!(
        serde_json::from_value::<RustCompanionCrate>(
            json!({"dependencies": {"serde": {"version": "=1.0.229", "git": "https://example.com/serde"}}})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<RustCompanionCrate>(json!({"dependencies": {"serde": 1}}))
            .is_err()
    );
}
