use super::RustCrateBuild;
use crate::native::rust::Linkage;
use crate::native::rust_companion_crate::RustCompanionCrate;
use serde_json::json;
use std::path::{Path, PathBuf};

struct RustCrateBuildTests;

impl RustCrateBuildTests {
    const COMPANION: &'static str = r#"#[allow(non_snake_case)]
mod TachyonRustServices {
    use super::*;
    pub mod models {
        pub struct Greeting {
            pub text: String,
        }
    }
    pub struct GreetingService {}
    impl GreetingService {
        pub fn greet() -> models::Greeting {
            models::Greeting { text: tiny_greeting::hello() }
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn tac_native_ping() -> i32 {
    7
}
"#;
    const ENTRY: &'static str = r#"use crate::models::Greeting;
use crate::GreetingService;

pub struct ProbeMain {}

impl ProbeMain {
    pub fn run() -> i32 {
        let greeting: Greeting = GreetingService::greet();
        println!("{} from {}", greeting.text, tiny_platform::name());
        0
    }
}
"#;
    const CHECKSUM: &'static str = "{\"files\":{},\"package\":null}";

    fn write(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap_or(Path::new("."))).unwrap_or_default();
        std::fs::write(path, contents)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    }

    fn platform() -> &'static str {
        if cfg!(windows) {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        }
    }

    fn declaration(entry: &str) -> RustCompanionCrate {
        serde_json::from_value(json!({
            "dependencies": {"tiny-greeting": "=1.0.0"},
            "platformDependencies": {Self::platform(): {"tiny-platform": "=2.1.0"}},
            "executables": {"probe-helper": entry}
        }))
        .unwrap_or_else(|error| panic!("declaration: {error}"))
    }

    fn vendor_crate(vendor: &Path, name: &str, version: &str, body: &str) {
        let folder = vendor.join(format!("{name}-{version}"));
        Self::write(
            &folder.join("Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        );
        Self::write(&folder.join("src/lib.rs"), body);
        Self::write(&folder.join(".cargo-checksum.json"), Self::CHECKSUM);
    }

    fn project(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
        let source = root.join("stage/project/companion.rs");
        Self::write(&source, Self::COMPANION);
        let client = root.join("app/client");
        Self::write(&client.join("shared/native/ProbeMain.rs"), Self::ENTRY);
        let vendor = root.join("app/vendor/rust");
        Self::vendor_crate(
            &vendor,
            "tiny-greeting",
            "1.0.0",
            "pub fn hello() -> String { String::from(\"hello\") }\n",
        );
        Self::vendor_crate(
            &vendor,
            "tiny-platform",
            "2.1.0",
            &format!(
                "pub fn name() -> &'static str {{ \"{}\" }}\n",
                Self::platform()
            ),
        );
        (source, client, vendor)
    }

    fn build<'build>(
        declaration: &'build RustCompanionCrate,
        paths: &'build (PathBuf, PathBuf, PathBuf),
        target: PathBuf,
    ) -> RustCrateBuild<'build> {
        RustCrateBuild {
            declaration,
            source: &paths.0,
            client_root: &paths.1,
            vendor: &paths.2,
            target,
        }
    }
}

#[test]
fn the_cargo_build_is_offline_and_reads_crates_only_from_the_vendor_folder() {
    let declaration = RustCrateBuildTests::declaration("/shared/native/ProbeMain.rs");
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
    let paths = RustCrateBuildTests::project(root.path());
    let build = RustCrateBuildTests::build(&declaration, &paths, root.path().join("target"));
    let arguments = build
        .cargo_arguments(
            Path::new("Cargo.toml"),
            Path::new("target"),
            Some("aarch64-pc-windows-msvc"),
        )
        .unwrap_or_default();
    for expected in [
        "--offline",
        "net.offline=true",
        "--release",
        "aarch64-pc-windows-msvc",
        "source.crates-io.replace-with=\"tachyon-vendored\"",
    ] {
        assert!(
            arguments.iter().any(|argument| argument == expected),
            "{expected}: {arguments:?}"
        );
    }
    let directory = format!(
        "source.tachyon-vendored.directory=\"{}\"",
        paths.2.display()
    );
    assert!(arguments.contains(&directory), "{arguments:?}");
    let helpers_only: RustCompanionCrate = serde_json::from_value(
        json!({"executables": {"probe-helper": "/shared/native/ProbeMain.rs"}}),
    )
    .unwrap_or_default();
    let plain = RustCrateBuildTests::build(&helpers_only, &paths, root.path().join("target"));
    let arguments = plain
        .cargo_arguments(Path::new("Cargo.toml"), Path::new("target"), None)
        .unwrap_or_default();
    assert!(arguments.iter().any(|argument| argument == "--offline"));
    assert!(
        !arguments
            .iter()
            .any(|argument| argument.contains("replace-with"))
    );
}

#[test]
fn staging_writes_the_manifest_the_helper_main_and_the_library_exports() {
    let declaration = RustCrateBuildTests::declaration("/shared/native/ProbeMain.rs");
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
    let paths = RustCrateBuildTests::project(root.path());
    let build = RustCrateBuildTests::build(&declaration, &paths, root.path().join("target"));
    let manifest = build
        .stage(Linkage::Shared)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(manifest.ends_with("stage/project/rust-companion/Cargo.toml"));
    let main = std::fs::read_to_string(manifest.with_file_name("bin").join("probe-helper.rs"))
        .unwrap_or_default();
    assert!(main.contains("use tachyon_companion::*;"), "{main}");
    assert!(
        main.contains("#[path = \"probe-helper-entry.rs\"]"),
        "{main}"
    );
    let entry =
        std::fs::read_to_string(manifest.with_file_name("bin").join("probe-helper-entry.rs"))
            .unwrap_or_default();
    assert!(entry.contains("pub struct ProbeMain"), "{entry}");
    let cargo = std::fs::read_to_string(&manifest).unwrap_or_default();
    assert!(cargo.contains("path = \"../companion.rs\""), "{cargo}");
    assert!(cargo.contains("path = \"bin/probe-helper.rs\""), "{cargo}");
    assert!(
        main.contains("std::process::exit(entry::ProbeMain::run());"),
        "{main}"
    );
    let source = std::fs::read_to_string(&paths.0).unwrap_or_default();
    assert!(source.ends_with("pub use crate::TachyonRustServices::*;\n"));
    assert_eq!(
        RustCrateBuild::library_file(Linkage::Shared, Some("aarch64-pc-windows-msvc"), true),
        "tachyon_companion.dll"
    );
    assert_eq!(
        RustCrateBuild::library_file(Linkage::Shared, Some("x86_64-unknown-linux-gnu"), false),
        "libtachyon_companion.so"
    );
    assert_eq!(
        RustCrateBuild::library_file(Linkage::Static, Some("aarch64-apple-darwin"), false),
        "libtachyon_companion.a"
    );
}

#[test]
fn a_helper_entry_without_a_run_method_is_refused_with_the_contract() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
    let paths = RustCrateBuildTests::project(root.path());
    RustCrateBuildTests::write(
        &paths.1.join("shared/native/ProbeMain.rs"),
        "pub struct ProbeMain {}\nimpl ProbeMain { pub fn start() {} }\n",
    );
    let declaration = RustCrateBuildTests::declaration("/shared/native/ProbeMain.rs");
    let build = RustCrateBuildTests::build(&declaration, &paths, root.path().join("target"));
    let error = build
        .stage(Linkage::Shared)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(error.contains("pub fn run() -> i32"), "{error}");
}

#[test]
fn declared_crates_without_a_vendor_folder_ask_for_cargo_vendor_and_never_download() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
    let mut paths = RustCrateBuildTests::project(root.path());
    paths.2 = root.path().join("app/vendor/missing");
    let declaration = RustCrateBuildTests::declaration("/shared/native/ProbeMain.rs");
    let build = RustCrateBuildTests::build(&declaration, &paths, root.path().join("target"));
    let error = build
        .require_vendored_crates(Path::new("Cargo.toml"))
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(
        error.contains("cargo vendor --manifest-path Cargo.toml"),
        "{error}"
    );
    assert!(error.contains("offline"), "{error}");
}

#[tokio::test]
async fn a_vendored_platform_crate_and_a_helper_executable_build_offline_and_run() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
    let paths = RustCrateBuildTests::project(root.path());
    let declaration = RustCrateBuildTests::declaration("/shared/native/ProbeMain.rs");
    let build = RustCrateBuildTests::build(&declaration, &paths, root.path().join("target"));
    build
        .stage(Linkage::Shared)
        .unwrap_or_else(|error| panic!("{error}"));
    let bin = root.path().join("bundle/bin");
    let library = bin.join("companion-library");
    let placed = build
        .build(Linkage::Shared, None, &library, &bin)
        .await
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(library.is_file());
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    assert_eq!(placed, vec![bin.join(format!("probe-helper{suffix}"))]);
    let output = std::process::Command::new(&placed[0])
        .output()
        .unwrap_or_else(|error| panic!("helper runs: {error}"));
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("hello from {}", RustCrateBuildTests::platform())
    );
}
