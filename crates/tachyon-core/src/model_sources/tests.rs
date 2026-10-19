#![allow(clippy::expect_used)]

use crate::node_driver::NodeDriver;
use crate::tac_web::WebLanguage;
use std::path::{Path, PathBuf};
use std::process::Command;

#[tokio::test]
async fn rust_private_field_model_roundtrips_through_the_owned_handler() {
    use crate::{HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions};
    use tachyon_contracts::{HandlerBody, HandlerBodyEncoding, HandlerRequest, HttpMethod};
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/models/PrivateInput.rs",
        "pub struct PrivateInput { value: i64 }\n",
    );
    let entry = "server/controllers/InputController.rs";
    write(
        root.path(),
        entry,
        "use crate::models::PrivateInput;\n#[RequestRoute(\"/input\")]\n#[Public]\nstruct InputController {}\nimpl InputController {\n fn new() -> Self { Self {} }\n #[PostRoute]\n fn post_input(&self, #[FromBody] input: PrivateInput) -> PrivateInput { input }\n}\n",
    );
    let source = HandlerSource::discover(root.path(), entry).expect("owned compiler");
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let mut request = HandlerRequest::route("private-model", "/input", HttpMethod::Post);
    request.arguments = Some(vec![serde_json::json!({"value": 7})]);
    request.body = Some(HandlerBody {
        encoding: HandlerBodyEncoding::Utf8,
        data: "{\"value\":7}".into(),
    });
    let response = supervisor
        .invoke(&source, &request, &HandlerCancellation::default())
        .await
        .expect("owned runtime");
    assert!(response.error.is_none(), "{:?}", response.error);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&response.body.expect("body").data)
            .expect("JSON"),
        serde_json::json!({"value": 7})
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("server/models/PrivateInput.rs"))
            .expect("authored"),
        "pub struct PrivateInput { value: i64 }\n"
    );
}

#[tokio::test]
async fn rust_model_that_imports_json_itself_compiles_with_yon_codecs() {
    use crate::{HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions};
    use tachyon_contracts::{HandlerRequest, HttpMethod};
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/models/Summary.rs",
        "use crate::Json;\n\npub struct Summary {\n    pub summary: Json,\n}\n",
    );
    let entry = "server/controllers/SummaryController.rs";
    write(
        root.path(),
        entry,
        "use crate::models::Summary;\n#[RequestRoute(\"/summary\")]\n#[Public]\nstruct SummaryController {}\nimpl SummaryController {\n fn new() -> Self { Self {} }\n #[GetRoute]\n fn get_summary(&self) -> Summary { Summary { summary: Json::parse(\"{\\\"id\\\":7}\").expect(\"parse\") } }\n}\n",
    );
    let source = HandlerSource::discover(root.path(), entry).expect("owned compiler");
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let request = HandlerRequest::route("json-import", "/summary", HttpMethod::Get);
    let response = supervisor
        .invoke(&source, &request, &HandlerCancellation::default())
        .await
        .expect("owned runtime");
    assert!(response.error.is_none(), "{:?}", response.error);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&response.body.expect("body").data)
            .expect("JSON"),
        serde_json::json!({"summary": {"id": 7}})
    );
}

#[test]
fn rust_yon_codecs_are_owned_by_private_field_model_modules() {
    let root = tempfile::tempdir().expect("project");
    let path = root.path().join("server/models/PrivateInput.rs");
    std::fs::create_dir_all(path.parent().expect("models")).expect("directory");
    let source = "#[Checked]\npub struct PrivateInput {\n #[Min(1)]\n value: i64,\n}\n";
    std::fs::write(&path, source).expect("model");
    let models = super::ModelSources::read(root.path(), "server", "rs").expect("schema");
    let program = models.add_to_yon_program("struct Handler;", true);
    let module = program
        .split("mod model_0 {")
        .nth(1)
        .expect("owning namespace");
    assert!(module.contains("use super::yon_codec_names::*;"));
    assert!(program.contains("pub(super) use crate::{Json, YonRead};"));
    assert!(module.contains("impl YonRead for PrivateInput"));
    assert!(module.contains("impl From<PrivateInput> for Json"));
    assert!(module.contains("value.value"));
    assert!(module.contains("value: i64"));
    assert!(!module.contains("pub value"));
    assert!(!program.contains("impl YonRead for crate::models::"));
    assert!(
        crate::route_types::Converters::program_text("rs", root.path())
            .expect("server codecs")
            .is_empty()
    );
    assert_eq!(std::fs::read_to_string(path).expect("authored"), source);
}

#[test]
fn rust_plain_handler_and_native_models_keep_their_prior_projection() {
    let root = tempfile::tempdir().expect("project");
    let path = root.path().join("server/models/PrivateInput.rs");
    std::fs::create_dir_all(path.parent().expect("models")).expect("directory");
    std::fs::write(path, "pub struct PrivateInput { value: i64 }\n").expect("model");
    let models = super::ModelSources::read(root.path(), "server", "rs").expect("plain model");
    let plain = models.add_to_program("struct Handler;");
    assert_eq!(models.add_to_yon_program("struct Handler;", false), plain);
    assert!(!plain.contains("YonRead"));
    assert!(!plain.contains("Json"));
    assert_eq!(
        models.rust_module(),
        "#[allow(dead_code, unused_imports)]\npub mod models {\npub use self::model_0::*;\n#[doc = \"server/models/PrivateInput.rs\"]\nmod model_0 {\npub struct PrivateInput { value: i64 }\n\n}\n}\n"
    );
}

#[test]
fn java_yon_checked_models_preserve_metadata_before_sdk_projection() {
    let root = tempfile::tempdir().expect("project");
    let path = root.path().join("server/models/OrderInput.java");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("models");
    let source = "import java.util.List;\n\n@Checked\nclass OrderInput {\n    @Required\n    String name;\n}\n";
    std::fs::write(&path, source).expect("model");
    let models = super::ModelSources::read(root.path(), "server", "java").expect("schema model");
    let entry = Path::new("server/services/OrderService.java");
    let authored = models.metadata().assembled(entry, "@Service\nclass OrderService {\n    Object accept(@Check(OrderInput) Object order) { return order; }\n}\n").expect("metadata projection");
    let lowered = crate::behavior::lower(entry, &authored, "yon")
        .expect("schema lowering")
        .expect("annotated source");
    assert!(lowered.contains("case \"OrderInput\": return new Rule[]"));
    assert!(lowered.contains("new Rule(\"name\", true, false, null, null, null, null)"));
    let role = models
        .metadata()
        .without_declarations(entry, &lowered)
        .expect("data separated");
    assert!(!role.contains("class OrderInput"));
    let sdk = models.add_to_program(&role);
    assert!(!sdk.contains("@Checked") && !sdk.contains("@Required"));
    assert!(sdk.contains("class OrderInput") && sdk.contains("String name"));
}

#[test]
fn groovy_metadata_removes_schema_declarations_after_an_import() {
    let path = Path::new("server/controllers/ConfigurationController.groovy");
    let schema = "@Settings({\"prefix\":\"YON_CATALOG_\"})\nclass CatalogSettings {\n    int pageSize = 20\n}\n";
    let controller = "@RequestRoute(\"/configuration\")\nclass ConfigurationController {\n}\n";
    let metadata = super::ModelMetadata::new(vec![(
        PathBuf::from("server/models/CatalogSettings.groovy"),
        schema.to_owned(),
    )]);
    let composed = format!("import models.Configuration\n{schema}\n{controller}");
    let retained = metadata
        .without_declarations(path, &composed)
        .expect("schema separated");
    assert!(!retained.contains("class CatalogSettings"));
    assert!(retained.contains("import models.Configuration"));
    assert!(retained.contains(controller));
}

#[test]
fn javascript_metadata_removes_only_injected_schema_declarations() {
    let path = Path::new("client/services/InputService.js");
    let schema = "@Checked\nclass ProductInput {\n  @Required\n  name = '';\n}\n";
    let service =
        "@Service\nexport default class InputService { accept(value) { return value; } }\n";
    let metadata = super::ModelMetadata::new(vec![(
        PathBuf::from("client/models/ProductInput.js"),
        schema.to_owned(),
    )]);
    let composed = format!("{schema}\n{service}");
    let retained = metadata
        .without_declarations(path, &composed)
        .expect("schema separated");
    assert!(!retained.contains("class ProductInput"));
    assert!(retained.contains(service));
    assert_eq!(retained.len(), composed.len());
}

#[test]
fn lua_schema_models_are_read_in_both_realms_and_role_markers_stay_refused() {
    let root = tempfile::tempdir().expect("project");
    let source = "-- @Checked\n---@class OrderInput\nOrderInput = {\n  -- @Required\n  ---@type string\n  name = nil,\n}\n";
    for realm in ["client", "server"] {
        let path = root.path().join(format!("{realm}/models/OrderInput.lua"));
        std::fs::create_dir_all(path.parent().expect("parent")).expect("models");
        std::fs::write(&path, source).expect("schema model");
        let models = super::ModelSources::read(root.path(), realm, "lua")
            .unwrap_or_else(|failure| panic!("{realm}: {failure}"));
        assert!(
            models.joined_sources().contains("OrderInput = {"),
            "{realm}"
        );
    }
    let path = root.path().join("server/models/OrderInput.lua");
    std::fs::write(&path, source.replace("-- @Checked", "-- @Service")).expect("role model");
    let failure = super::ModelSources::read(root.path(), "server", "lua")
        .err()
        .expect("a role marker on a Lua model stays refused");
    assert_eq!(failure.diagnostics()[0].code.number(), 1310);
    assert_eq!(
        failure.diagnostics()[0].spans[0].file,
        "server/models/OrderInput.lua"
    );
}

struct Sources;

impl Sources {
    const RUST_MODEL: &str = "#[derive(Clone, Debug)]\npub struct Guide {\n    pub title: String,\n    pub pages: i64,\n}\n";
    const RUST_SERVICE: &str = "use crate::models::Guide;\n\n#[Service()]\n#[derive(Default)]\nstruct GuideService {\n    pages: i64,\n}\n\nimpl GuideService {\n    pub fn read(&mut self) -> String {\n        let guide = Guide { title: String::from(\"Guide\"), pages: 3 };\n        self.pages = guide.pages;\n        format!(\"{} {}\", guide.title, guide.pages)\n    }\n}\n";
    const CSHARP_MODEL: &str = "public record Guide(string Title, int Pages);\n";
    const CSHARP_SHELF_MODEL: &str = "using System.Collections.Generic;\n\npublic class Shelf\n{\n    public List<Guide> Guides { get; init; } = new();\n}\n";
    const CSHARP_SERVICE: &str = "using Models;\n\n[Service()]\npublic class GuideService\n{\n    public int Pages = 0;\n\n    public string Read()\n    {\n        var shelf = new Shelf { Guides = { new Guide(\"Guide\", 3) } };\n        Pages = shelf.Guides[0].Pages;\n        return shelf.Guides[0].Title + \" \" + Pages;\n    }\n}\n";
    const GO_MODEL: &str = "type Guide struct {\n\tTitle string\n\tPages int\n}\n";
    const GO_SERVICE: &str = "package main\n\nimport \"strconv\"\n\n// @Service()\ntype GuideService struct {\n\tPages int\n}\n\nfunc (service *GuideService) Read() string {\n\tguide := Guide{Title: \"Guide\", Pages: 3}\n\tservice.Pages = guide.Pages\n\treturn guide.Title + \" \" + strconv.Itoa(guide.Pages)\n}\n";
    const RUBY_MODEL: &str = "# @!attribute [r] title\n#   @return [String]\n# @!attribute [r] pages\n#   @return [Integer]\nGuide = Data.define(:title, :pages)\n";
    const RUBY_SERVICE: &str = "#[Service]\nclass GuideService\n  attr_reader :pages\n\n  def read\n    guide = Guide.new(title: \"Guide\", pages: 3)\n    @pages = guide.pages\n    \"#{guide.title} #{guide.pages}\"\n  end\nend\n";
    const LUA_MODEL: &str =
        "---@class Guide\n---@field title string\n---@field pages integer\nGuide = {}\n";
    const PYTHON_MODEL: &str = "from dataclasses import dataclass\n\n@dataclass\nclass Guide:\n    title: str\n    pages: int\n";
    const PYTHON_SERVICE: &str = "from models import Guide\n\n@service\nclass GuideService:\n    pages: int = 0\n\n    def read(self) -> str:\n        guide = Guide(title='Guide', pages=3)\n        self.pages = guide.pages\n        return guide.title + ' ' + str(guide.pages)\n";
    const LUA_SERVICE: &str = "-- @Service\n---@class GuideService\nGuideService = {\n  ---@type integer\n  pages = 0,\n}\n\n---@return string\nfunction GuideService:read()\n  local guide = setmetatable({ title = \"Guide\", pages = 3 }, Guide)\n  self.pages = guide.pages\n  return guide.title .. \" \" .. guide.pages\nend\n";
    const CPP_MODEL: &str = "struct Guide {\n    std::string title;\n    int pages = 0;\n};\n";
    const CPP_SERVICE: &str = "class [[Service]] GuideService {\npublic:\n    int pages = 0;\n\n    std::string read() {\n        Guide guide{\"Guide\", 3};\n        pages = guide.pages;\n        return guide.title + \" \" + std::to_string(guide.pages);\n    }\n};\n";
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
    let output =
        NodeDriver::output(NodeDriver::command().arg("drive.js").current_dir(&dist)).expect("node");
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
        WebLanguage::Python => (Sources::PYTHON_MODEL, Sources::PYTHON_SERVICE),
        WebLanguage::Cpp => (Sources::CPP_MODEL, Sources::CPP_SERVICE),
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
async fn a_cpp_web_assembly_service_uses_a_model() {
    if !available("em++", "--version") || !available("node", "--version") {
        eprintln!("skipping: em++ (Emscripten) or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    build_web_service(root.path(), WebLanguage::Cpp, "hpp").await;
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
    let ready = library.is_some_and(|path| path.is_file()) && available("kotlinc-js", "-version");
    assert!(
        ready || std::env::var_os("TAC_KOTLIN_WEB_REQUIRE_TOOLCHAINS").is_none(),
        "kotlinc-js and TAC_KOTLIN_WASM_STDLIB are required on this runner"
    );
    ready
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
    let published = crate::tac_dart::build_all(
        root.path(),
        &services,
        &instances,
        crate::BuildProfile::Release,
    )
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

#[test]
fn a_script_service_gets_only_the_models_it_names_and_the_models_they_name() {
    let root = tempfile::tempdir().expect("project");
    for (relative, source) in [
        (
            "client/models/Order.js",
            "/**\n * @typedef {Object} Order\n * @property {OrderLine[]} lines\n */\nexport {}\n",
        ),
        (
            "client/models/OrderLine.js",
            "/**\n * @typedef {Object} OrderLine\n * @property {Money} price\n */\nexport {}\n",
        ),
        (
            "client/models/Money.js",
            "/** @typedef {number} Money */\nexport {}\n",
        ),
        (
            "client/models/Unrelated.js",
            "/**\n * @typedef {Object} Unrelated\n * @property {string} note\n */\nexport {}\n",
        ),
        (
            "client/models/ProductInput.js",
            "@Checked\nexport default class ProductInput {\n  @Required\n  name;\n}\n",
        ),
    ] {
        write(root.path(), relative, source);
    }
    let models = super::ModelSources::read(root.path(), "client", "js").expect("models");
    let program = "export default class Orders {\n  /** @param {Order} order */\n  place(order) { return order; }\n}\n";
    let selected = models.used_by([program, program]).add_to_program(program);
    for kept in ["{OrderLine[]} lines", "{Money} price", "{number} Money"] {
        assert!(selected.contains(kept), "{kept}: {selected}");
    }
    assert!(!selected.contains("Unrelated"), "{selected}");
    assert!(!selected.contains("class ProductInput"), "{selected}");
    assert!(selected.ends_with(program), "{selected}");
    let checked = "export default class Inputs { make() { return new ProductInput(); } }\n";
    let selected = models.used_by(["", checked]).add_to_program(checked);
    assert!(selected.contains("class ProductInput"), "{selected}");
    assert!(!selected.contains("@typedef"), "{selected}");
    assert_eq!(
        models.used_by(["", "plain"]).add_to_program("plain"),
        "plain"
    );
}

#[test]
fn a_typescript_service_keeps_the_field_types_of_the_models_it_names() {
    let root = tempfile::tempdir().expect("project");
    for (relative, source) in [
        (
            "client/models/Guide.ts",
            "export interface Guide {\n  section: Section\n}\n",
        ),
        (
            "client/models/Section.ts",
            "export interface Section {\n  title: string\n}\n",
        ),
        (
            "client/models/Other.ts",
            "export interface Other {\n  count: number\n}\n",
        ),
    ] {
        write(root.path(), relative, source);
    }
    let models = super::ModelSources::read(root.path(), "client", "ts").expect("models");
    let program = "export default class Guides { read(guide: Guide): string { return guide.section.title; } }\n";
    let selected = models.used_by([program, program]).add_to_program(program);
    assert!(selected.contains("interface Guide"), "{selected}");
    assert!(selected.contains("interface Section"), "{selected}");
    assert!(!selected.contains("interface Other"), "{selected}");
}
