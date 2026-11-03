#![allow(clippy::expect_used)]

use std::path::Path;
use tachyon_contracts::NativeTarget;

struct NativeCfgImplTests;

impl NativeCfgImplTests {
    const SERVICE: &'static str = "#[Service]\n#[derive(Default)]\nstruct PlatformService {\n    name: String,\n}\n\n#[cfg(target_os = \"linux\")]\nimpl PlatformService {\n    const NAME: &'static str = \"linux\";\n\n    fn detect(&mut self) {\n        self.name = String::from(Self::NAME);\n    }\n}\n\n#[cfg(windows)]\nimpl PlatformService {\n    const NAME: &'static str = \"windows\";\n\n    fn detect(&mut self) {\n        self.name = String::from(Self::NAME);\n    }\n}\n\n#[cfg(not(any(target_os = \"linux\", windows)))]\nimpl PlatformService {\n    const NAME: &'static str = \"other\";\n\n    fn detect(&mut self) {\n        self.name = String::from(Self::NAME);\n    }\n}\n";
    const COMPONENT: &'static str = "<main>\n  <h1>Platform</h1>\n  <p>{platform.name}</p>\n  <button on:click=\"platform.detect()\">Detect</button>\n</main>\n";
    const MANIFEST: &'static str = r#"{"name":"Platform","start_url":"/","tac":{"application":{"id":"com.example.platform","version":"1.0.0"}}}"#;
    const TARGETS: [(NativeTarget, &'static str); 2] = [
        (NativeTarget::Linux, "x86_64-unknown-linux-gnu"),
        (NativeTarget::Windows, "x86_64-pc-windows-msvc"),
    ];

    fn write(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, contents).expect("fixture");
    }

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        let client = root.path().join("client");
        Self::write(&client.join("manifest.json"), Self::MANIFEST);
        Self::write(&client.join("services/PlatformService.rs"), Self::SERVICE);
        Self::write(
            &client.join("components/platform/panel/tac.html"),
            Self::COMPONENT,
        );
        Self::write(
            &client.join("pages/tac.html"),
            "<platform-panel hydrate=\"load\" />\n",
        );
        root
    }

    fn target_installed(triple: &str) -> bool {
        std::process::Command::new("rustc")
            .args(["--print", "target-libdir", "--target", triple])
            .output()
            .is_ok_and(|output| {
                output.status.success()
                    && Path::new(String::from_utf8_lossy(&output.stdout).trim()).is_dir()
            })
    }
}

#[test]
fn same_named_methods_in_exclusive_cfg_impl_blocks_pass_the_layout_check() {
    crate::annotation_layout::validate(
        Path::new("client/services/PlatformService.rs"),
        NativeCfgImplTests::SERVICE,
    )
    .expect("cfg-gated impl blocks of the file's own type");
}

#[tokio::test]
async fn same_named_methods_in_exclusive_cfg_impl_blocks_build_and_type_check_for_linux_and_windows()
 {
    for (target, triple) in NativeCfgImplTests::TARGETS {
        let root = NativeCfgImplTests::project();
        let options = super::compiler::NativeBuildOptions {
            output_directory: root.path().join("dist"),
            target,
            package: false,
            ..Default::default()
        };
        let built = super::compiler::NativeCompiler::build_async(root.path(), &options)
            .await
            .unwrap_or_else(|error| panic!("{target:?}: {error}"));
        let companion = built.output_directory().join("project/companion.rs");
        let source = std::fs::read_to_string(&companion).expect("companion");
        assert_eq!(
            source.matches("fn detect(&mut self)").count(),
            3,
            "{target:?}"
        );
        if !NativeCfgImplTests::target_installed(triple) {
            eprintln!("skipping the {triple} type check: its standard library is not installed");
            continue;
        }
        let output = std::process::Command::new("rustc")
            .args([
                "--edition",
                "2024",
                "--crate-type",
                "cdylib",
                "--emit",
                "metadata",
                "--target",
                triple,
                "-o",
            ])
            .arg(root.path().join(format!("{triple}.rmeta")))
            .arg(&companion)
            .output()
            .expect("rustc runs");
        assert!(
            output.status.success(),
            "{triple}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
