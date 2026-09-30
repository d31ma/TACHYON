#![allow(clippy::expect_used)]

use crate::tac_web::WebLanguage;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Sources;

impl Sources {
    const RUST_MODEL: &str = "#[derive(Clone, Debug)]\npub struct Guide {\n    pub title: String,\n    pub pages: i64,\n}\n";
    const RUST_SERVICE: &str = "use crate::models::Guide;\n\n#[Service()]\n#[derive(Default)]\nstruct GuideService {\n    pages: i64,\n}\n\nimpl GuideService {\n    fn read(&mut self) -> String {\n        let guide = Guide { title: String::from(\"Guide\"), pages: 3 };\n        self.pages = guide.pages;\n        format!(\"{} {}\", guide.title, guide.pages)\n    }\n}\n";
    const CSHARP_MODEL: &str = "public record Guide(string Title, int Pages);\n";
    const CSHARP_SHELF_MODEL: &str = "using System.Collections.Generic;\n\npublic class Shelf\n{\n    public List<Guide> Guides { get; init; } = new();\n}\n";
    const CSHARP_SERVICE: &str = "using Models;\n\n[Service()]\npublic class GuideService\n{\n    public int Pages = 0;\n\n    public string Read()\n    {\n        var shelf = new Shelf { Guides = { new Guide(\"Guide\", 3) } };\n        Pages = shelf.Guides[0].Pages;\n        return shelf.Guides[0].Title + \" \" + Pages;\n    }\n}\n";
    const GO_MODEL: &str = "type Guide struct {\n\tTitle string\n\tPages int\n}\n";
    const GO_SERVICE: &str = "package main\n\nimport \"strconv\"\n\n// @Service()\ntype GuideService struct {\n\tPages int\n}\n\nfunc (service *GuideService) Read() string {\n\tguide := Guide{Title: \"Guide\", Pages: 3}\n\tservice.Pages = guide.Pages\n\treturn guide.Title + \" \" + strconv.Itoa(guide.Pages)\n}\n";
    const RUBY_MODEL: &str = "# @!attribute [r] title\n#   @return [String]\n# @!attribute [r] pages\n#   @return [Integer]\nGuide = Data.define(:title, :pages)\n";
    const RUBY_SERVICE: &str = "#[Service]\nclass GuideService\n  attr_reader :pages\n\n  def read\n    guide = Guide.new(title: \"Guide\", pages: 3)\n    @pages = guide.pages\n    \"#{guide.title} #{guide.pages}\"\n  end\nend\n";
    const LUA_MODEL: &str =
        "---@class Guide\n---@field title string\n---@field pages integer\nGuide = {}\n";
    const LUA_SERVICE: &str = "-- @Service\n---@class GuideService\nGuideService = {\n  ---@type integer\n  pages = 0,\n}\n\n---@return string\nfunction GuideService:read()\n  local guide = setmetatable({ title = \"Guide\", pages = 3 }, Guide)\n  self.pages = guide.pages\n  return guide.title .. \" \" .. guide.pages\nend\n";
    const KOTLIN_MODEL: &str = "data class Guide(val title: String, val pages: Int)\n";
    const KOTLIN_SERVICE: &str = "import models.Guide\n\n@Service\nclass GuideService {\n    var pages: Int = 0\n\n    fun read(): String {\n        val guide = Guide(\"Guide\", 3)\n        pages = guide.pages\n        return guide.title + \" \" + guide.pages\n    }\n}\n";
    const SWIFT_MODEL: &str = "struct Guide {\n    let title: String\n    let pages: Int\n}\n";
    const JAVA_MODEL: &str = "record Guide(String title, int pages) {}\n";
    const JAVA_SERVICE: &str = "import models.Guide;\n\n@Service\nclass GuideService {\n    int pages = 0;\n\n    String read() {\n        Guide guide = new Guide(\"Guide\", 3);\n        pages = guide.pages();\n        return guide.title() + \" \" + guide.pages();\n    }\n}\n";
    const SWIFT_SERVICE: &str = "@Service\nfinal class GuideService {\n    var pages: Int = 0\n\n    func read() -> String {\n        let guide = Guide(title: \"Guide\", pages: 3)\n        pages = guide.pages\n        return guide.title + \" \" + String(guide.pages)\n    }\n}\n";
    const FILE_FETCH: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n";
    const DART_MODEL: &str = "class Guide {\n  final String title;\n  final int pages;\n\n  const Guide(this.title, this.pages);\n}\n";
    const DART_GUIDE_SERVICE: &str = "import 'models/Guide.dart';\n\n@Service()\nclass GuideService {\n  Guide guide() => const Guide('Guide', 3);\n}\n";
    const DART_SHELF_SERVICE: &str = "import 'models/Guide.dart';\n\n@Service()\nclass ShelfService {\n  late GuideService guide;\n  int pages = 0;\n\n  String read() {\n    final Guide shared = guide.guide();\n    pages = shared.pages;\n    return '${shared.title} ${shared.pages}';\n  }\n}\n";
    const WEB_DRIVER: &str = "import GuideService from './GuideService.js'\n\
         const service = new GuideService({})\n\
         const read = service.read()\n\
         console.log(JSON.stringify({ read, pages: service.pages }))\n\
         process.exit(0)\n";
    const DART_DRIVER: &str = "const { default: GuideService } = await import('./GuideService.js')\n\
         const { default: ShelfService } = await import('./ShelfService.js')\n\
         const shelf = new ShelfService({})\n\
         shelf.guide = new GuideService({})\n\
         const read = shelf.read()\n\
         console.log(JSON.stringify({ read, pages: shelf.pages }))\n";
}

fn available(program: &str, flag: &str) -> bool {
    Command::new(program)
        .arg(flag)
        .output()
        .is_ok_and(|output| output.status.success())
}

fn web_assembly_target_installed() -> bool {
    let Ok(output) = Command::new("rustc")
        .args(["--print", "target-libdir", "--target"])
        .arg(crate::native::RustCompanion::WEB_TARGET)
        .output()
    else {
        return false;
    };
    let directory = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    output.status.success() && Path::new(&directory).is_dir()
}

fn dotnet_web_workload_installed() -> bool {
    let Ok(output) = Command::new("dotnet").args(["workload", "list"]).output() else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout).contains("wasm-tools")
}

fn write(root: &Path, relative: &str, source: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(&path, source).expect("source");
    path
}

fn publish(root: &Path, published: Vec<(String, Vec<u8>)>) {
    for (name, bytes) in published {
        assert!(!name.contains("models"), "{name}");
        let path = root.join("dist").join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, bytes).expect("publish");
    }
    let package = root.join("dist/package.json");
    std::fs::write(package, "{\"type\":\"module\"}").expect("package");
}

fn run_driver(root: &Path, driver: &str) -> serde_json::Value {
    let dist = root.join("dist");
    std::fs::write(dist.join("drive.js"), driver).expect("driver");
    let output = Command::new("node")
        .arg("drive.js")
        .current_dir(&dist)
        .output()
        .expect("node");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_str(stdout.trim()).expect("report")
}

async fn build_web_service(root: &Path, language: WebLanguage, extension: &str) {
    let (model, service) = match language {
        WebLanguage::Rust => (Sources::RUST_MODEL, Sources::RUST_SERVICE),
        WebLanguage::CSharp => (Sources::CSHARP_MODEL, Sources::CSHARP_SERVICE),
        WebLanguage::Go => (Sources::GO_MODEL, Sources::GO_SERVICE),
        WebLanguage::Ruby => (Sources::RUBY_MODEL, Sources::RUBY_SERVICE),
        WebLanguage::Kotlin => (Sources::KOTLIN_MODEL, Sources::KOTLIN_SERVICE),
        WebLanguage::Swift => (Sources::SWIFT_MODEL, Sources::SWIFT_SERVICE),
        WebLanguage::Java => (Sources::JAVA_MODEL, Sources::JAVA_SERVICE),
        WebLanguage::Lua => (Sources::LUA_MODEL, Sources::LUA_SERVICE),
    };
    write(root, &format!("client/models/Guide.{extension}"), model);
    if matches!(language, WebLanguage::CSharp) {
        write(root, "client/models/Shelf.cs", Sources::CSHARP_SHELF_MODEL);
    }
    let source = write(
        root,
        &format!("client/services/GuideService.{extension}"),
        service,
    );
    let services = [(
        String::from("guide"),
        String::from("GuideService"),
        language,
        source,
    )];
    let jobs = crate::tac_web::BuildJobs::new(1);
    let published = crate::tac_web::build_all(root, &services, &jobs, crate::BuildProfile::Release)
        .await
        .expect("the web service builds with its models");
    publish(root, published.files);
}

#[tokio::test]
async fn a_rust_web_assembly_service_uses_a_model() {
    if !web_assembly_target_installed() || !available("node", "--version") {
        eprintln!("skipping: the wasm32 Rust target or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    build_web_service(root.path(), WebLanguage::Rust, "rs").await;
    let report = run_driver(root.path(), Sources::WEB_DRIVER);
    assert_eq!(report, serde_json::json!({"read": "Guide 3", "pages": 3}));
}

#[tokio::test]
async fn a_csharp_web_assembly_service_uses_a_model() {
    if !dotnet_web_workload_installed() || !available("node", "--version") {
        eprintln!("skipping: the .NET wasm-tools workload or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    build_web_service(root.path(), WebLanguage::CSharp, "cs").await;
    let report = run_driver(root.path(), Sources::WEB_DRIVER);
    assert_eq!(report, serde_json::json!({"read": "Guide 3", "pages": 3}));
}

#[tokio::test]
async fn a_go_web_assembly_service_uses_a_model() {
    if !available("go", "version") || !available("node", "--version") {
        assert!(
            std::env::var_os("CI").is_none(),
            "CI must provide go and node"
        );
        eprintln!("skipping: go or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    build_web_service(root.path(), WebLanguage::Go, "go").await;
    let report = run_driver(root.path(), &fetching_driver());
    assert_eq!(report, serde_json::json!({"read": "Guide 3", "pages": 3}));
}

#[tokio::test]
async fn a_ruby_web_service_uses_a_model() {
    if !available("node", "--version") {
        assert!(std::env::var_os("CI").is_none(), "CI must provide node");
        eprintln!("skipping: node is unavailable");
        return;
    }
    if let Some(reason) = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    build_web_service(root.path(), WebLanguage::Ruby, "rb").await;
    let report = run_driver(root.path(), &fetching_driver());
    assert_eq!(report, serde_json::json!({"read": "Guide 3", "pages": 3}));
}

#[tokio::test]
async fn a_lua_web_service_uses_a_model() {
    if !available("node", "--version") {
        assert!(std::env::var_os("CI").is_none(), "CI must provide node");
        eprintln!("skipping: node is unavailable");
        return;
    }
    if let Some(reason) = crate::tac_lua::LuaRuntimeBuild::unavailable_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    build_web_service(root.path(), WebLanguage::Lua, "lua").await;
    let report = run_driver(root.path(), &fetching_driver());
    assert_eq!(report, serde_json::json!({"read": "Guide 3", "pages": 3}));
}

#[tokio::test]
async fn a_kotlin_web_assembly_service_uses_a_model() {
    if !kotlin_web_ready() || !available("node", "--version") {
        eprintln!("skipping: kotlinc-js, the Kotlin/Wasm standard library or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    build_web_service(root.path(), WebLanguage::Kotlin, "kt").await;
    let report = run_driver(root.path(), Sources::WEB_DRIVER);
    assert_eq!(report, serde_json::json!({"read": "Guide 3", "pages": 3}));
}

#[tokio::test]
async fn a_swift_web_assembly_service_uses_a_model() {
    if !crate::tac_web::swift_tests::swift_web_ready() {
        eprintln!("skipping: the Swift SDK for WebAssembly or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    build_web_service(root.path(), WebLanguage::Swift, "swift").await;
    let report = run_driver(root.path(), &fetching_driver());
    assert_eq!(report, serde_json::json!({"read": "Guide 3", "pages": 3}));
}

#[tokio::test]
async fn a_java_web_assembly_service_uses_a_model() {
    if !crate::native::JavaWebServices::teavm_installed()
        || !available("javac", "-version")
        || !available("node", "--version")
    {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    build_web_service(root.path(), WebLanguage::Java, "java").await;
    let report = run_driver(root.path(), &fetching_driver());
    assert_eq!(report, serde_json::json!({"read": "Guide 3", "pages": 3}));
}

fn fetching_driver() -> String {
    let driver = Sources::WEB_DRIVER.replace(
        "import GuideService from './GuideService.js'\n",
        "const { default: GuideService } = await import('./GuideService.js')\n",
    );
    format!("{}{driver}", Sources::FILE_FETCH)
}

fn kotlin_web_ready() -> bool {
    let library = std::env::var_os("TAC_KOTLIN_WASM_STDLIB").map(PathBuf::from);
    library.is_some_and(|path| path.is_file()) && available("kotlinc-js", "-version")
}

#[tokio::test]
async fn dart_browser_services_share_a_model() {
    if !available("dart", "--version") || !available("node", "--version") {
        eprintln!("skipping: dart or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    write(root.path(), "client/models/Guide.dart", Sources::DART_MODEL);
    let guide = write(
        root.path(),
        "client/services/GuideService.dart",
        Sources::DART_GUIDE_SERVICE,
    );
    let shelf = write(
        root.path(),
        "client/services/ShelfService.dart",
        Sources::DART_SHELF_SERVICE,
    );
    let services = [
        ("GuideService", guide.as_path()),
        ("ShelfService", shelf.as_path()),
    ];
    let instances = ["guide", "shelf"].map(String::from).into_iter().collect();
    let published = crate::tac_dart::build_all(root.path(), &services, &instances, crate::BuildProfile::Release)
        .await
        .expect("the Dart services build with their models");
    publish(root.path(), published);
    let driver = format!("{}{}", Sources::FILE_FETCH, Sources::DART_DRIVER);
    let report = run_driver(root.path(), &driver);
    assert_eq!(report, serde_json::json!({"read": "Guide 3", "pages": 3}));
}

#[test]
fn projects_without_models_keep_their_programs() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/models/Guide.ts",
        "export interface Guide {}\n",
    );
    for extension in ["rs", "cs", "py", "php", "java", "swift", "go"] {
        let models = super::ModelSources::read(root.path(), "server", extension).expect("read");
        assert!(!models.changes_program_text(), "{extension}");
        assert_eq!(models.add_to_program("program"), "program");
        assert!(models.separate_files().is_empty());
    }
    let typescript = super::ModelSources::read(root.path(), "server", "ts").expect("read");
    assert!(typescript.separate_files().is_empty());
}

#[test]
fn go_models_are_staged_as_files_of_package_main() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/models/Guide.go",
        "package main\n\ntype Guide struct {\n\tTitle string `json:\"title\"`\n}\n",
    );
    write(
        root.path(),
        "server/models/nested/Kind.go",
        "// kinds\ntype Kind string\n",
    );
    let models = super::ModelSources::read(root.path(), "server", "go").expect("read");
    assert!(!models.changes_program_text());
    assert_eq!(models.add_to_program("program"), "program");
    let staged = models.separate_files();
    assert_eq!(
        staged,
        vec![
            (
                PathBuf::from("model_0.go"),
                String::from(
                    "//line server/models/Guide.go:1\npackage main\n\ntype Guide struct {\n\tTitle string `json:\"title\"`\n}\n"
                ),
            ),
            (
                PathBuf::from("model_1.go"),
                String::from(
                    "//line server/models/nested/Kind.go:1\npackage main; // kinds\ntype Kind string\n"
                ),
            ),
        ]
    );
}

#[test]
fn a_go_model_with_behavior_is_refused_when_read() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/models/Guide.go",
        "type Guide struct{}\n\nfunc (Guide) Title() string { return \"\" }\n",
    );
    let Err(failure) = super::ModelSources::read(root.path(), "client", "go") else {
        panic!("a Go model method must be refused");
    };
    assert_eq!(failure.diagnostics()[0].code.number(), 1310);
}
