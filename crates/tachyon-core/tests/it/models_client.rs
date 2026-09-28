#![allow(clippy::expect_used)]

use std::path::Path;
use std::process::{Command, Stdio};

struct TestUnit {
    extension: &'static str,
    program: &'static str,
    configuration: &'static str,
    model: &'static str,
    source: &'static str,
}

impl TestUnit {
    const RUST: Self = Self {
        extension: "rs",
        program: "rustc",
        configuration: "#[Setup()]\nstruct CatalogSetup {}\nimpl CatalogSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> { Ok(BehaviorConfiguration::default()) }\n}\n",
        model: "#[derive(Clone, Debug)]\npub struct Guide {\n    pub title: String,\n    pub pages: u32,\n}\n",
        source: "use crate::models::Guide;\n\n#[Test(CatalogSetup, {\"partition\":\"tests\"})]\nstruct CatalogServiceTest { behaviors: std::cell::RefCell<Option<BehaviorContext>> }\nimpl CatalogServiceTest {\n    fn new() -> Self { Self { behaviors: std::cell::RefCell::new(None) } }\n    fn testUsesAModel(&self) {\n        let guide = Guide { title: String::from(\"Guide\"), pages: 3 };\n        Assert::equals(3, guide.pages);\n        Assert::equals(\"Guide\", guide.title);\n    }\n}\n",
    };

    const CSHARP: Self = Self {
        extension: "cs",
        program: "dotnet",
        configuration: "[Setup]\npublic sealed class CatalogSetup {\n    public static BehaviorAdapters configure(BehaviorExecution execution) { return new BehaviorAdapters(); }\n}\n",
        model: "public record Guide(string Title, int Pages);\n",
        source: "using Models;\n\n[Test(typeof(CatalogSetup), {\"partition\":\"tests\"})]\nsealed class CatalogServiceTest {\n    void testUsesAModel() {\n        var guide = new Guide(\"Guide\", 3);\n        if (guide.Pages != 3 || guide.Title != \"Guide\") throw new System.Exception(\"wrong guide\");\n    }\n}\n",
    };

    const DART: Self = Self {
        extension: "dart",
        program: "dart",
        configuration: "@Setup()\nclass CatalogSetup {\n  static BehaviorAdapters configure(BehaviorExecution execution) {\n    return BehaviorAdapters();\n  }\n}\n",
        model: "class Guide {\n  final String title;\n  final int pages;\n\n  const Guide(this.title, this.pages);\n}\n",
        source: "import 'models/Guide.dart';\n\n@Test(CatalogSetup, {\"partition\":\"tests\"})\nclass CatalogServiceTest {\n  Future<void> testUsesAModel() async {\n    const guide = Guide('Guide', 3);\n    if (guide.pages != 3 || guide.title != 'Guide') throw StateError('wrong guide');\n  }\n}\n",
    };

    const KOTLIN: Self = Self {
        extension: "kt",
        program: "kotlinc",
        configuration: "@Setup\nclass CatalogSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n",
        model: "data class Guide(val title: String, val pages: Int)\n",
        source: "import models.Guide\n\n@Test(CatalogSetup::class, {\"partition\":\"tests\"})\nclass CatalogServiceTest {\n    fun testUsesAModel() {\n        val guide = Guide(\"Guide\", 3)\n        if (guide.pages != 3 || guide.title != \"Guide\") throw IllegalStateException(\"wrong guide\")\n    }\n}\n",
    };

    const SWIFT: Self = Self {
        extension: "swift",
        program: "swiftc",
        configuration: "@Setup()\nfinal class CatalogSetup {\n    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {\n        var adapters = BehaviorAdapters()\n        adapters.authorize = { _, _ in true }\n        return adapters\n    }\n}\n",
        model: "import Foundation\n\nstruct Guide: Codable, Equatable {\n    let title: String\n    var pages: Int = 0\n}\n",
        source: "@Test(CatalogSetup, {\"partition\":\"tests\"})\nfinal class CatalogServiceTest {\n    func testUsesAModel() async throws {\n        let guide = Guide(title: \"Guide\", pages: 3)\n        if guide.pages != 3 || guide.title != \"Guide\" { throw BehaviorError(\"WRONG_GUIDE\") }\n    }\n}\n",
    };
}

fn available(program: &str) -> bool {
    let flag = if program == "kotlinc" {
        "-version"
    } else {
        "--version"
    };
    Command::new(program)
        .arg(flag)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
    std::fs::write(path, source).expect("source");
}

fn assert_unit_uses_a_model(unit: &TestUnit) {
    if !available(unit.program) {
        eprintln!(
            "skipping {}: {} is unavailable",
            unit.extension, unit.program
        );
        return;
    }
    let extension = unit.extension;
    let root = tempfile::tempdir().expect("project");
    let configuration = format!("client/setups/CatalogSetup.{extension}");
    write(root.path(), &configuration, unit.configuration);
    write(
        root.path(),
        &format!("client/models/Guide.{extension}"),
        unit.model,
    );
    let relative = format!("client/tests/CatalogServiceTest.{extension}");
    write(root.path(), &relative, unit.source);
    let report = tachyon_core::client_tests::run(root.path(), Path::new(&relative))
        .unwrap_or_else(|failure| panic!("{extension}: {failure:?}"));
    assert_eq!(report.cases.len(), 1, "{extension}: {:?}", report.cases);
    assert!(report.cases[0].passed, "{extension}: {:?}", report.cases);
}

#[test]
fn a_rust_test_unit_uses_a_model() {
    assert_unit_uses_a_model(&TestUnit::RUST);
}

#[test]
fn a_csharp_test_unit_uses_a_model() {
    assert_unit_uses_a_model(&TestUnit::CSHARP);
}

#[test]
fn a_dart_test_unit_uses_a_model() {
    assert_unit_uses_a_model(&TestUnit::DART);
}

#[test]
fn a_kotlin_test_unit_uses_a_model() {
    assert_unit_uses_a_model(&TestUnit::KOTLIN);
}

#[test]
fn a_swift_test_unit_uses_a_model() {
    assert_unit_uses_a_model(&TestUnit::SWIFT);
}

fn write_go_model_project(root: &Path, model: &str) {
    write(root, "client/pages/tac.html", "<guide-view />");
    write(
        root,
        "client/components/guide/view/tac.html",
        "<main>Guide</main>",
    );
    write(root, "client/models/Guide.go", model);
}

#[test]
fn a_go_client_model_is_validated_and_never_shipped() {
    let root = tempfile::tempdir().expect("project");
    write_go_model_project(
        root.path(),
        "package main\n\ntype Guide struct {\n\tTitle string `json:\"title\"`\n\tPages int    `json:\"pages\"`\n}\n",
    );
    let project = tachyon_core::ProjectDiscovery::discover(root.path()).expect("a Go model");
    tachyon_core::WebCompiler::build_project(&project, &tachyon_core::BuildOptions::default())
        .expect("build");
    assert_no_output_holds(&root.path().join("dist"), "Guide struct");
}

fn assert_no_output_holds(directory: &Path, text: &str) {
    for entry in std::fs::read_dir(directory).expect("output").flatten() {
        let path = entry.path();
        if path.is_dir() {
            assert_no_output_holds(&path, text);
            continue;
        }
        let bytes = std::fs::read(&path).expect("output file");
        let content = String::from_utf8_lossy(&bytes);
        assert!(!content.contains(text), "{}", path.display());
    }
}

#[test]
fn a_go_client_model_with_behavior_fails_project_discovery() {
    let root = tempfile::tempdir().expect("project");
    write_go_model_project(
        root.path(),
        "type Guide struct{}\n\nfunc (Guide) Title() string { return \"Guide\" }\n",
    );
    let failure =
        tachyon_core::ProjectDiscovery::discover(root.path()).expect_err("a Go model method");
    let rendered = failure.to_string();
    assert!(rendered.contains("TY1310"), "{rendered}");
    assert!(
        rendered.contains("A Go models file declares only"),
        "{rendered}"
    );
}
