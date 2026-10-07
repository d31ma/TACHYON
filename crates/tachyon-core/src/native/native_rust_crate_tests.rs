#![allow(clippy::expect_used)]

use serde_json::json;
use std::path::{Path, PathBuf};
use tachyon_contracts::NativeTarget;

struct NativeRustCrateTests;

impl NativeRustCrateTests {
    const MODEL: &'static str = "pub struct Greeting {\n    pub text: String,\n}\n";
    const SERVICE: &'static str = "#[Service]\n#[derive(Default)]\nstruct GreetingService {\n    text: String,\n}\n\nimpl GreetingService {\n    fn greet(&mut self) {\n        self.text = tiny_greeting::hello();\n    }\n}\n";
    const COMPONENT: &'static str = "<main>\n  <h1>Greeter</h1>\n  <p>{greeting.text}</p>\n  <button on:click=\"greeting.greet()\">Greet</button>\n</main>\n";
    const PAGE: &'static str = "<greeter-panel hydrate=\"load\" />\n";
    const HELPER: &'static str = "use crate::models::Greeting;\n\npub struct ProbeHelperMain {}\n\nimpl ProbeHelperMain {\n    const SUCCESS: i32 = 0;\n\n    pub fn run() -> i32 {\n        let greeting: Greeting = Greeting { text: tiny_greeting::hello() };\n        let platform: &str = tiny_platform::name();\n        println!(\"{} from {platform}\", greeting.text);\n        Self::SUCCESS\n    }\n}\n";
    const CHECKSUM: &'static str = "{\"files\":{},\"package\":null}";

    fn write(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, contents).expect("fixture");
    }

    fn platform(target: NativeTarget) -> &'static str {
        match target {
            NativeTarget::Windows => "windows",
            NativeTarget::Macos => "macos",
            _ => "linux",
        }
    }

    fn vendor_crate(root: &Path, name: &str, body: &str) {
        let folder = root.join("vendor/rust").join(name);
        Self::write(
            &folder.join("Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\nversion = \"1.0.0\"\nedition = \"2021\"\n"),
        );
        Self::write(&folder.join("src/lib.rs"), body);
        Self::write(&folder.join(".cargo-checksum.json"), Self::CHECKSUM);
    }

    fn project(target: NativeTarget) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        let platform = Self::platform(target);
        let manifest = json!({
            "name": "Greeter",
            "start_url": "/",
            "tac": {"application": {
                "id": "com.example.greeter",
                "version": "1.0.0",
                "rust": {
                    "dependencies": {"tiny-greeting": "=1.0.0"},
                    "platformDependencies": {platform: {"tiny-platform": "=1.0.0"}},
                    "executables": {"probe-helper": "/shared/native/ProbeHelperMain.rs"}
                }
            }}
        });
        let client = root.path().join("client");
        Self::write(&client.join("manifest.json"), &manifest.to_string());
        Self::write(&client.join("models/Greeting.rs"), Self::MODEL);
        Self::write(&client.join("services/GreetingService.rs"), Self::SERVICE);
        Self::write(
            &client.join("components/greeter/panel/tac.html"),
            Self::COMPONENT,
        );
        Self::write(&client.join("pages/tac.html"), Self::PAGE);
        Self::write(
            &client.join("shared/native/ProbeHelperMain.rs"),
            Self::HELPER,
        );
        Self::vendor_crate(
            root.path(),
            "tiny-greeting",
            "pub fn hello() -> String { String::from(\"hello\") }\n",
        );
        Self::vendor_crate(
            root.path(),
            "tiny-platform",
            &format!("pub fn name() -> &'static str {{ \"{platform}\" }}\n"),
        );
        root
    }

    async fn build(root: &Path, target: NativeTarget, package: bool) -> PathBuf {
        let options = super::compiler::NativeBuildOptions {
            output_directory: root.join("dist"),
            target,
            package,
            ..Default::default()
        };
        super::compiler::NativeCompiler::build_async(root, &options)
            .await
            .expect("native build with a declared Rust crate")
            .output_directory()
            .to_path_buf()
    }

    fn run_helper(helper: &Path, target: NativeTarget) {
        let output = std::process::Command::new(helper)
            .output()
            .expect("the helper runs");
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            format!("hello from {}", Self::platform(target))
        );
    }
}

#[tokio::test]
async fn a_declared_rust_crate_is_staged_beside_the_companion_for_every_desktop() {
    for target in [
        NativeTarget::Linux,
        NativeTarget::Windows,
        NativeTarget::Macos,
    ] {
        let root = NativeRustCrateTests::project(target);
        let output = NativeRustCrateTests::build(root.path(), target, false).await;
        let project = output.join("project/rust-companion");
        let manifest = std::fs::read_to_string(project.join("Cargo.toml")).expect("Cargo.toml");
        assert!(
            manifest.contains("tiny-greeting = { version = \"=1.0.0\" }"),
            "{manifest}"
        );
        assert!(
            manifest.contains("tiny-platform = { version = \"=1.0.0\" }"),
            "{manifest}"
        );
        assert!(
            manifest.contains("[[bin]]\nname = \"probe-helper\""),
            "{manifest}"
        );
        let kinds = if target == NativeTarget::Macos {
            "[\"staticlib\", \"rlib\"]"
        } else {
            "[\"cdylib\", \"rlib\"]"
        };
        assert!(manifest.contains(kinds), "{target:?}: {manifest}");
        let main =
            std::fs::read_to_string(project.join("bin/probe-helper.rs")).expect("helper main");
        assert!(main.contains("entry::ProbeHelperMain::run()"), "{main}");
        let companion =
            std::fs::read_to_string(output.join("project/companion.rs")).expect("companion");
        assert!(companion.contains("pub use crate::TachyonRustServices::*;"));
    }
}

#[tokio::test]
async fn a_rust_declaration_without_a_rust_service_is_refused() {
    let root = NativeRustCrateTests::project(NativeTarget::Linux);
    std::fs::remove_file(root.path().join("client/services/GreetingService.rs")).expect("remove");
    NativeRustCrateTests::write(
        &root.path().join("client/components/greeter/panel/tac.html"),
        "<main><h1>Greeter</h1></main>\n",
    );
    NativeRustCrateTests::write(
        &root.path().join("client/pages/tac.html"),
        "<greeter-panel />\n",
    );
    let options = super::compiler::NativeBuildOptions {
        output_directory: root.path().join("dist"),
        target: NativeTarget::Linux,
        package: false,
        ..Default::default()
    };
    let error = super::compiler::NativeCompiler::build_async(root.path(), &options)
        .await
        .expect_err("no Rust service")
        .to_string();
    assert!(error.contains("no Rust service"), "{error}");
}

#[tokio::test]
async fn a_helper_named_like_the_app_executable_is_refused() {
    let root = NativeRustCrateTests::project(NativeTarget::Windows);
    let manifest = root.path().join("client/manifest.json");
    let text = std::fs::read_to_string(&manifest).expect("manifest");
    std::fs::write(&manifest, text.replace("probe-helper", "greeter")).expect("rename");
    let options = super::compiler::NativeBuildOptions {
        output_directory: root.path().join("dist"),
        target: NativeTarget::Windows,
        package: false,
        ..Default::default()
    };
    let error = super::compiler::NativeCompiler::build_async(root.path(), &options)
        .await
        .expect_err("name clash")
        .to_string();
    assert!(
        error.contains("the name of the app's own executable"),
        "{error}"
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Builds the full Linux host (GTK 4, WebKitGTK 6.0, cargo); run explicitly on the Linux lane"]
async fn the_linux_bundle_places_the_helper_beside_the_host_and_it_runs() {
    let root = NativeRustCrateTests::project(NativeTarget::Linux);
    let output = NativeRustCrateTests::build(root.path(), NativeTarget::Linux, true).await;
    let bin = output.join("Greeter/bin");
    assert!(bin.join("Greeter").is_file() && bin.join("libtachyoncompanion.so").is_file());
    NativeRustCrateTests::run_helper(&bin.join("probe-helper"), NativeTarget::Linux);
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "Builds and signs the full macOS host twice using existing system SQLite; run explicitly on the Mac lane"]
async fn static_native_link_macos_sqlite_archive_and_helper_build_repeatedly() {
    let root = NativeRustCrateTests::project(NativeTarget::Macos);
    let crate_folder = root.path().join("vendor/rust/tiny-greeting/src/lib.rs");
    NativeRustCrateTests::write(
        &crate_folder,
        r#"
pub fn hello() -> String {
    #[link(name = "sqlite3")]
    unsafe extern "C" {
        fn sqlite3_libversion_number() -> i32;
    }
    assert!(unsafe { sqlite3_libversion_number() } > 0);
    String::from("hello")
}
"#,
    );
    for _ in 0..2 {
        let output = NativeRustCrateTests::build(root.path(), NativeTarget::Macos, true).await;
        let bundle = output.join("Greeter.app");
        let helper = bundle.join("Contents/MacOS/probe-helper");
        let helper_path = helper.to_str().expect("helper path");
        let text = super::host::run_tool(helper_path, &[])
            .await
            .expect("SQLite helper runs");
        assert_eq!(text.trim(), "hello from macos");
        let bundle_path = bundle.to_str().expect("bundle path");
        super::host::run_tool(
            "/usr/bin/codesign",
            &["--verify", "--deep", "--strict", bundle_path],
        )
        .await
        .expect("the complete host and helper signature is valid");
    }
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "Builds and signs the full macOS host (swiftc, codesign, cargo); run explicitly on the Mac lane"]
async fn the_macos_bundle_places_the_signed_helper_in_contents_macos_and_it_runs() {
    let root = NativeRustCrateTests::project(NativeTarget::Macos);
    let output = NativeRustCrateTests::build(root.path(), NativeTarget::Macos, true).await;
    let bundle = output.join("Greeter.app");
    let helper = bundle.join("Contents/MacOS/probe-helper");
    NativeRustCrateTests::run_helper(&helper, NativeTarget::Macos);
    let verified = std::process::Command::new("/usr/bin/codesign")
        .args(["--verify", "--strict"])
        .arg(&bundle)
        .status()
        .expect("codesign runs");
    assert!(verified.success());
}

#[cfg(target_os = "windows")]
#[tokio::test]
#[ignore = "Builds the full Windows host (MSVC, WebView2 SDK, cargo); run explicitly in a developer prompt"]
async fn the_windows_bundle_places_the_helper_beside_the_host_and_it_runs() {
    let root = NativeRustCrateTests::project(NativeTarget::Windows);
    let output = NativeRustCrateTests::build(root.path(), NativeTarget::Windows, true).await;
    let bin = output.join("Greeter/bin");
    assert!(bin.join("Greeter.exe").is_file() && bin.join("TachyonRustCompanion.dll").is_file());
    NativeRustCrateTests::run_helper(&bin.join("probe-helper.exe"), NativeTarget::Windows);
}
