#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::BuildProfile;
use crate::node_driver::NodeDriver;
use crate::template::bound_attribute_tests::BoundAttributeDriver;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) struct AriaMenu {
    language: WebLanguage,
    extension: &'static str,
    source: &'static str,
    profile: BuildProfile,
    ready: fn() -> bool,
}

impl AriaMenu {
    const RUNTIME: &'static str = include_str!("../template/tac-web.js");

    const FETCH: &'static str = "import { readFile } from 'node:fs/promises'\n\
         import { fileURLToPath } from 'node:url'\n\
         globalThis.fetch = async (url) => new Response(await readFile(String(url).startsWith('file:') ? fileURLToPath(url) : url))\n\
         ;(0, eval)(await readFile('./tac-web.js', 'utf8'))\n";

    const TOGGLE: &'static str = "const { default: MenuService } = await import('./MenuService.js')\n\
         const menu = new MenuService({})\n\
         const read = () => [BoundAttribute.text('aria-expanded', menu.expanded), typeof menu.expanded]\n\
         const closed = read()\n\
         await menu.toggle()\n\
         const opened = read()\n\
         console.log(JSON.stringify({ closed, opened }))\n\
         process.exit(0)\n";

    const RUST: Self = Self {
        language: WebLanguage::Rust,
        extension: "rs",
        source: include_str!(
            "../../../../website/content/docs/templates/aria-states-from-boolean-fields/client/services/MenuService.rs"
        ),
        profile: BuildProfile::Release,
        ready: Self::rust_ready,
    };

    const GO: Self = Self {
        language: WebLanguage::Go,
        extension: "go",
        source: include_str!(
            "../../../../website/content/docs/templates/aria-states-from-boolean-fields/client/services/MenuService.go"
        ),
        profile: BuildProfile::Release,
        ready: Self::go_ready,
    };

    const CSHARP: Self = Self {
        language: WebLanguage::CSharp,
        extension: "cs",
        source: include_str!(
            "../../../../website/content/docs/templates/aria-states-from-boolean-fields/client/services/MenuService.cs"
        ),
        profile: BuildProfile::Development,
        ready: Self::dotnet_ready,
    };

    const FSHARP: Self = Self {
        language: WebLanguage::CSharp,
        extension: "fs",
        source: include_str!(
            "../../../../website/content/docs/templates/aria-states-from-boolean-fields/client/services/MenuService.fs"
        ),
        profile: BuildProfile::Development,
        ready: super::fsharp_tests::fsharp_web_ready,
    };

    const KOTLIN: Self = Self {
        language: WebLanguage::Kotlin,
        extension: "kt",
        source: include_str!(
            "../../../../website/content/docs/templates/aria-states-from-boolean-fields/client/services/MenuService.kt"
        ),
        profile: BuildProfile::Development,
        ready: super::kotlin_tests::kotlin_web_ready,
    };

    const SWIFT: Self = Self {
        language: WebLanguage::Swift,
        extension: "swift",
        source: include_str!(
            "../../../../website/content/docs/templates/aria-states-from-boolean-fields/client/services/MenuService.swift"
        ),
        profile: BuildProfile::Development,
        ready: super::swift_tests::swift_web_ready,
    };

    const JAVA: Self = Self {
        language: WebLanguage::Java,
        extension: "java",
        source: include_str!(
            "../../../../website/content/docs/templates/aria-states-from-boolean-fields/client/services/MenuService.java"
        ),
        profile: BuildProfile::Development,
        ready: super::java_tests::java_web_ready,
    };

    const CPP: Self = Self {
        language: WebLanguage::Cpp,
        extension: "hpp",
        source: include_str!(
            "../../../../website/content/docs/templates/aria-states-from-boolean-fields/client/services/MenuService.hpp"
        ),
        profile: BuildProfile::Release,
        ready: Self::cpp_ready,
    };

    const RUBY: Self = Self {
        language: WebLanguage::Ruby,
        extension: "rb",
        source: include_str!(
            "../../../../website/content/docs/templates/aria-states-from-boolean-fields/client/services/MenuService.rb"
        ),
        profile: BuildProfile::Release,
        ready: Self::ruby_ready,
    };

    const LUA: Self = Self {
        language: WebLanguage::Lua,
        extension: "lua",
        source: include_str!(
            "../../../../website/content/docs/templates/aria-states-from-boolean-fields/client/services/MenuService.lua"
        ),
        profile: BuildProfile::Release,
        ready: Self::lua_ready,
    };

    pub(crate) const PYTHON: &'static str = include_str!(
        "../../../../website/content/docs/templates/aria-states-from-boolean-fields/client/services/MenuService.py"
    );

    const DART: &'static str = include_str!(
        "../../../../website/content/docs/templates/aria-states-from-boolean-fields/client/services/MenuService.dart"
    );

    pub(crate) fn driver() -> String {
        format!("{}{}", BoundAttributeDriver::rule(), Self::TOGGLE)
    }

    pub(crate) fn assert_true_or_false(report: &serde_json::Value) {
        assert_eq!(
            report,
            &serde_json::json!({ "closed": ["false", "boolean"], "opened": ["true", "boolean"] })
        );
    }

    async fn report(&self) -> Option<serde_json::Value> {
        if !NodeDriver::available() || !(self.ready)() {
            eprintln!(
                "skipping: node or the web toolchain for .{} is unavailable",
                self.extension
            );
            return None;
        }
        let root = tempfile::tempdir().expect("project");
        let relative = format!("client/services/MenuService.{}", self.extension);
        let path = Self::write(root.path(), &relative, self.source.as_bytes());
        let services = [(
            String::from("menu"),
            String::from("MenuService"),
            self.language,
            path,
        )];
        let built = build_all(root.path(), &services, &BuildJobs::new(1), self.profile)
            .await
            .expect("the service builds");
        for (name, bytes) in built.files {
            Self::write(root.path(), &name, &bytes);
        }
        Some(Self::drive(root.path()))
    }

    async fn dart_report() -> Option<serde_json::Value> {
        if !NodeDriver::available() || !Self::answers("dart", "--version") {
            eprintln!("skipping: node or dart is unavailable");
            return None;
        }
        let root = tempfile::tempdir().expect("project");
        let path = Self::write(
            root.path(),
            "client/services/MenuService.dart",
            Self::DART.as_bytes(),
        );
        let services = [("MenuService", path.as_path())];
        let instances = std::collections::BTreeSet::from([String::from("menu")]);
        let modules =
            crate::tac_dart::build_all(root.path(), &services, &instances, BuildProfile::Release)
                .await
                .expect("the service builds");
        for (name, bytes) in modules {
            Self::write(root.path(), &name, &bytes);
        }
        Self::write(root.path(), "package.json", b"{\"type\":\"module\"}");
        Some(Self::drive(root.path()))
    }

    fn drive(root: &Path) -> serde_json::Value {
        Self::write(root, "tac-web.js", Self::RUNTIME.as_bytes());
        let script = format!("{}{}", Self::FETCH, Self::driver());
        Self::write(root, "drive.mjs", script.as_bytes());
        let output = NodeDriver::output(NodeDriver::command().arg("drive.mjs").current_dir(root))
            .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.lines().last().unwrap_or_default()).expect("report")
    }

    fn write(root: &Path, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(&path, bytes).expect("file");
        path
    }

    fn answers(program: &str, argument: &str) -> bool {
        Command::new(program)
            .arg(argument)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn rust_ready() -> bool {
        Command::new("rustc")
            .args(["--print", "target-libdir", "--target"])
            .arg(crate::native::RustCompanion::WEB_TARGET)
            .output()
            .is_ok_and(|output| Path::new(String::from_utf8_lossy(&output.stdout).trim()).is_dir())
    }

    fn go_ready() -> bool {
        Self::answers("go", "version")
    }

    fn dotnet_ready() -> bool {
        Command::new("dotnet")
            .args(["workload", "list"])
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("wasm-tools"))
    }

    fn cpp_ready() -> bool {
        crate::program_probe::ProgramProbe::answers("em++", "--version")
    }

    fn ruby_ready() -> bool {
        crate::tac_ruby::RubyRuntimeBuild::unavailable_reason().is_none()
    }

    fn lua_ready() -> bool {
        crate::tac_lua::LuaRuntimeBuild::unavailable_reason().is_none()
    }
}

#[tokio::test]
async fn a_rust_bool_field_writes_aria_expanded_as_true_or_false() {
    if let Some(report) = AriaMenu::RUST.report().await {
        AriaMenu::assert_true_or_false(&report);
    }
}

#[tokio::test]
async fn a_go_bool_field_writes_aria_expanded_as_true_or_false() {
    if let Some(report) = AriaMenu::GO.report().await {
        AriaMenu::assert_true_or_false(&report);
    }
}

#[tokio::test]
async fn a_csharp_bool_field_writes_aria_expanded_as_true_or_false() {
    if let Some(report) = AriaMenu::CSHARP.report().await {
        AriaMenu::assert_true_or_false(&report);
    }
}

#[tokio::test]
async fn an_fsharp_bool_field_writes_aria_expanded_as_true_or_false() {
    if let Some(report) = AriaMenu::FSHARP.report().await {
        AriaMenu::assert_true_or_false(&report);
    }
}

#[tokio::test]
async fn a_kotlin_boolean_field_writes_aria_expanded_as_true_or_false() {
    if let Some(report) = AriaMenu::KOTLIN.report().await {
        AriaMenu::assert_true_or_false(&report);
    }
}

#[tokio::test]
async fn a_swift_bool_field_writes_aria_expanded_as_true_or_false() {
    if let Some(report) = AriaMenu::SWIFT.report().await {
        AriaMenu::assert_true_or_false(&report);
    }
}

#[tokio::test]
async fn a_java_boolean_field_writes_aria_expanded_as_true_or_false() {
    if let Some(report) = AriaMenu::JAVA.report().await {
        AriaMenu::assert_true_or_false(&report);
    }
}

#[tokio::test]
async fn a_cpp_bool_field_writes_aria_expanded_as_true_or_false() {
    if let Some(report) = AriaMenu::CPP.report().await {
        AriaMenu::assert_true_or_false(&report);
    }
}

#[tokio::test]
async fn a_ruby_boolean_field_writes_aria_expanded_as_true_or_false() {
    if let Some(report) = AriaMenu::RUBY.report().await {
        AriaMenu::assert_true_or_false(&report);
    }
}

#[tokio::test]
async fn a_lua_boolean_field_writes_aria_expanded_as_true_or_false() {
    if let Some(report) = AriaMenu::LUA.report().await {
        AriaMenu::assert_true_or_false(&report);
    }
}

#[tokio::test]
async fn a_dart_bool_field_writes_aria_expanded_as_true_or_false() {
    if let Some(report) = AriaMenu::dart_report().await {
        AriaMenu::assert_true_or_false(&report);
    }
}
