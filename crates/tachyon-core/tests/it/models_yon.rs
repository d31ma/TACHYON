#![allow(clippy::expect_used)]

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct Project {
    extension: &'static str,
    program: &'static str,
    files: &'static [(&'static str, &'static str)],
}

impl Project {
    const RUST: Self = Self {
        extension: "rs",
        program: "rustc",
        files: &[
            (
                "server/models/Guide.rs",
                "use crate::models::Kind;\n\n#[derive(Clone, Debug)]\npub struct Guide {\n    pub title: String,\n    pub pages: u32,\n    pub kind: Kind,\n}\n",
            ),
            (
                "server/models/Kind.rs",
                "#[derive(Clone, Copy, Debug)]\npub enum Kind {\n    Short,\n    Long,\n}\n",
            ),
            (
                "server/services/GuideService.rs",
                "use crate::models::{Guide, Kind};\n\n#[yon::service]\nstruct GuideService;\nimpl GuideService {\n    fn new() -> Self { Self }\n    fn guide(&self) -> Guide { Guide { title: String::from(\"Guide\"), pages: 3, kind: Kind::Long } }\n}\n",
            ),
            (
                "server/controllers/GuideController.rs",
                "use std::sync::Arc;\nuse crate::models::{Guide, Kind};\n\n#[RequestRoute(\"/guide\")]\n#[Public]\n#[yon::needs(GuideService)]\nstruct GuideController { guides: Arc<GuideService> }\nimpl GuideController {\n    fn new(guides: Arc<GuideService>) -> Self { Self { guides } }\n    #[GetRoute]\n    fn get_guide(&self) -> Json {\n        let guide: Guide = self.guides.guide();\n        let long = matches!(guide.kind, Kind::Long);\n        Json::map([(\"title\", Json::from(guide.title)), (\"pages\", Json::from(guide.pages)), (\"long\", Json::from(long))])\n    }\n}\n",
            ),
        ],
    };

    const CSHARP: Self = Self {
        extension: "cs",
        program: "dotnet",
        files: &[
            (
                "server/models/Guide.cs",
                "public record Guide(string Title, int Pages, Kind Kind);\n",
            ),
            (
                "server/models/Kind.cs",
                "public enum Kind { Short, Long }\n",
            ),
            (
                "server/models/Shelf.cs",
                "using System.Collections.Generic;\n\npublic class Shelf\n{\n    public List<Guide> Guides { get; init; } = new();\n}\n",
            ),
            (
                "server/services/GuideService.cs",
                "using Models;\n\n[Service]\nclass GuideService {\n    public Guide Guide() => new Guide(\"Guide\", 3, Kind.Long);\n}\n",
            ),
            (
                "server/controllers/GuideController.cs",
                "using Models;\n\n[RequestRoute(\"/guide\")]\n[Public]\n[Needs(typeof(GuideService))]\nclass GuideController {\n    readonly GuideService guides;\n    public GuideController(GuideService guides) { this.guides = guides; }\n    [GetRoute]\n    public object GetGuide() {\n        var shelf = new Shelf { Guides = { guides.Guide() } };\n        Guide guide = shelf.Guides[0];\n        return new { title = guide.Title, pages = guide.Pages, @long = guide.Kind == Kind.Long };\n    }\n}\n",
            ),
        ],
    };

    const DART: Self = Self {
        extension: "dart",
        program: "dart",
        files: &[
            (
                "server/models/Guide.dart",
                "import 'Kind.dart';\n\nclass Guide {\n  final String title;\n  final int pages;\n  final Kind kind;\n\n  const Guide(this.title, this.pages, this.kind);\n}\n",
            ),
            ("server/models/Kind.dart", "enum Kind { short, long }\n"),
            (
                "server/services/GuideService.dart",
                "import 'models/Guide.dart';\nimport 'models/Kind.dart';\n\n@Service\nclass GuideService {\n  Guide guide() => const Guide('Guide', 3, Kind.long);\n}\n",
            ),
            (
                "server/controllers/GuideController.dart",
                "import 'models/Guide.dart';\nimport 'models/Kind.dart';\n\n@RequestRoute(\"/guide\")\n@Public\nclass GuideController {\n  @Needs\n  final GuideService guideService;\n\n  @GetRoute\n  Future<Object?> getGuide() async {\n    final Guide guide = guideService.guide();\n    return {'title': guide.title, 'pages': guide.pages, 'long': guide.kind == Kind.long};\n  }\n}\n",
            ),
        ],
    };

    const KOTLIN: Self = Self {
        extension: "kt",
        program: "kotlinc",
        files: &[
            (
                "server/models/Guide.kt",
                "data class Guide(val title: String, val pages: Int, val kind: Kind)\n",
            ),
            ("server/models/Kind.kt", "enum class Kind { SHORT, LONG }\n"),
            ("server/models/Title.kt", "typealias Title = String\n"),
            (
                "server/services/GuideService.kt",
                "import models.Guide\nimport models.Kind\n\n@Service\nclass GuideService {\n    fun guide(): Guide = Guide(\"Guide\", 3, Kind.LONG)\n}\n",
            ),
            (
                "server/controllers/GuideController.kt",
                "import models.Guide\nimport models.Kind\nimport models.Title\n\n@RequestRoute(\"/guide\")\n@Public\n@Needs(GuideService::class)\nclass GuideController(val guides: GuideService) {\n    @GetRoute\n    fun getGuide(): Any? {\n        val guide: Guide = guides.guide()\n        val title: Title = guide.title\n        return mapOf(\"title\" to title, \"pages\" to guide.pages, \"long\" to (guide.kind == Kind.LONG))\n    }\n}\n",
            ),
        ],
    };

    const JAVA: Self = Self {
        extension: "java",
        program: "java",
        files: &[
            (
                "server/models/Guide.java",
                "import java.util.List;\n\nrecord Guide(String title, int pages, Kind kind, List<String> tags) {}\n",
            ),
            ("server/models/Kind.java", "enum Kind { SHORT, LONG }\n"),
            (
                "server/services/GuideService.java",
                "import models.Guide;\nimport models.Kind;\n\n@Service\nclass GuideService {\n    Guide guide() { return new Guide(\"Guide\", 3, Kind.LONG, java.util.List.of(\"a\")); }\n}\n",
            ),
            (
                "server/controllers/GuideController.java",
                "import models.Guide;\nimport models.Kind;\n\n@RequestRoute(\"/guide\")\n@Public\n@Needs({GuideService.class})\nclass GuideController {\n    final GuideService guides;\n    GuideController(GuideService guides) { this.guides = guides; }\n    @GetRoute\n    Object getGuide() {\n        Guide guide = guides.guide();\n        return java.util.Map.of(\"title\", guide.title(), \"pages\", guide.pages(), \"long\", guide.kind() == Kind.LONG);\n    }\n}\n",
            ),
        ],
    };

    const PYTHON: Self = Self {
        extension: "py",
        program: "python3",
        files: &[
            (
                "server/models/Guide.py",
                "from dataclasses import dataclass\n\nfrom models import Kind\n\n\n@dataclass\nclass Guide:\n    title: str\n    pages: int\n    kind: Kind\n",
            ),
            (
                "server/models/Kind.py",
                "from enum import Enum\n\n\nclass Kind(Enum):\n    SHORT = 'short'\n    LONG = 'long'\n",
            ),
            (
                "server/services/GuideService.py",
                "from models import Guide, Kind\n\n\n@service()\nclass GuideService:\n    def guide(self):\n        return Guide('Guide', 3, Kind.LONG)\n",
            ),
            (
                "server/controllers/GuideController.py",
                "from models import Kind\n\n\n@request_route(\"/guide\")\n@public\n@needs(GuideService)\nclass GuideController:\n    def __init__(self, guides):\n        self.guides = guides\n\n    @get_route\n    def get_guide(self):\n        guide = self.guides.guide()\n        return {'title': guide.title, 'pages': guide.pages, 'long': guide.kind is Kind.LONG}\n",
            ),
        ],
    };

    const GO: Self = Self {
        extension: "go",
        program: "go",
        files: &[
            (
                "server/models/Guide.go",
                "package main\n\ntype Guide struct {\n\tTitle string `json:\"title\"`\n\tPages int    `json:\"pages\"`\n\tKind  Kind   `json:\"-\"`\n\tLong  bool   `json:\"long\"`\n}\n",
            ),
            ("server/models/Kind.go", "type Kind string\n"),
            (
                "server/controllers/GuideController.go",
                "// @RequestRoute(\"/guide\")\n// @Public\ntype GuideController struct {\n}\n\n// @GetRoute\nfunc (controller *GuideController) GetGuide() any {\n\tlong := Kind(\"long\")\n\tguide := Guide{Title: \"Guide\", Pages: 3, Kind: long}\n\tguide.Long = guide.Kind == long\n\treturn guide\n}\n",
            ),
        ],
    };

    const PHP: Self = Self {
        extension: "php",
        program: "php-cgi",
        files: &[
            (
                "server/models/Guide.php",
                "<?php\nclass Guide\n{\n    public function __construct(public string $title, public int $pages, public Kind $kind) {}\n}\n",
            ),
            (
                "server/models/Kind.php",
                "<?php\nenum Kind: string\n{\n    case Short = 'short';\n    case Long = 'long';\n}\n",
            ),
            (
                "server/services/GuideService.php",
                "<?php\nuse Models\\Guide;\nuse Models\\Kind;\n\n#[Service]\nclass GuideService {\n    public function guide(): Guide { return new Guide('Guide', 3, Kind::Long); }\n}\n",
            ),
            (
                "server/controllers/GuideController.php",
                "<?php\nuse Models\\Guide;\nuse Models\\Kind;\n\n#[RequestRoute(\"/guide\")]\n#[Public]\n#[Needs(GuideService::class)]\nclass GuideController {\n    public function __construct(public GuideService $guides) {}\n    #[GetRoute]\n    public function getGuide() {\n        $guide = $this->guides->guide();\n        return ['title' => $guide->title, 'pages' => $guide->pages, 'long' => $guide->kind === Kind::Long];\n    }\n}\n",
            ),
        ],
    };
}

fn available(program: &str) -> bool {
    let flag = match program {
        "kotlinc" => "-version",
        "go" => "version",
        _ => "--version",
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

async fn get_guide(root: &Path, extension: &str) -> serde_json::Value {
    let relative = format!("server/controllers/GuideController.{extension}");
    let source = HandlerSource::discover(root, &relative)
        .unwrap_or_else(|failure| panic!("{extension}: {failure:?}"));
    let supervisor = HandlerSupervisor::new(HandlerSupervisorOptions {
        default_timeout: Duration::from_mins(2),
        ..HandlerSupervisorOptions::default()
    })
    .expect("supervisor");
    let response = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("models", "/guide", HttpMethod::Get),
            &HandlerCancellation::default(),
        )
        .await
        .expect("invocation");
    assert!(
        response.error.is_none(),
        "{extension}: {:?}",
        response.error
    );
    assert_eq!(response.status, 200, "{extension}: {response:?}");
    let body = response.body.expect("body").data;
    serde_json::from_str(&body).unwrap_or_else(|_| panic!("{extension}: {body}"))
}

async fn assert_guide_from_models(project: &Project) {
    if !available(project.program) {
        eprintln!(
            "skipping {}: {} is unavailable",
            project.extension, project.program
        );
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, source) in project.files {
        write(root.path(), relative, source);
    }
    let body = get_guide(root.path(), project.extension).await;
    assert_eq!(
        body,
        serde_json::json!({"title": "Guide", "pages": 3, "long": true}),
        "{}",
        project.extension
    );
}

#[tokio::test]
async fn a_rust_handler_uses_a_model() {
    assert_guide_from_models(&Project::RUST).await;
}

#[tokio::test]
async fn a_csharp_handler_uses_a_model() {
    assert_guide_from_models(&Project::CSHARP).await;
}

#[tokio::test]
async fn a_dart_handler_uses_a_model() {
    assert_guide_from_models(&Project::DART).await;
}

#[tokio::test]
async fn a_kotlin_handler_uses_a_model() {
    assert_guide_from_models(&Project::KOTLIN).await;
}

#[tokio::test]
async fn a_java_handler_uses_a_model() {
    assert_guide_from_models(&Project::JAVA).await;
}

#[tokio::test]
async fn a_python_handler_uses_a_model() {
    assert_guide_from_models(&Project::PYTHON).await;
}

#[tokio::test]
async fn a_php_handler_uses_a_model() {
    assert_guide_from_models(&Project::PHP).await;
}

#[tokio::test]
async fn a_go_handler_uses_a_model() {
    go_is_required_in_ci();
    assert_guide_from_models(&Project::GO).await;
}

#[test]
fn a_go_model_compile_error_names_the_models_file() {
    let failure = go_guide_failure("type Guide struct {\n\tTitle string\n\tTitle string\n}\n");
    let Some(rendered) = failure else {
        return;
    };
    assert!(rendered.contains("server/models/Guide.go:3"), "{rendered}");
    assert!(rendered.contains("Title redeclared"), "{rendered}");
}

#[test]
fn a_go_model_field_with_an_unknown_type_names_the_models_file() {
    let failure = go_guide_failure("type Guide struct {\n\tTitle string\n\tShelf Missing\n}\n");
    let Some(rendered) = failure else {
        return;
    };
    assert!(rendered.contains("server/models/Guide.go"), "{rendered}");
    assert!(rendered.contains("Missing"), "{rendered}");
}

fn go_guide_failure(model: &str) -> Option<String> {
    go_is_required_in_ci();
    if !available("go") {
        return None;
    }
    let root = tempfile::tempdir().expect("project");
    write(root.path(), "server/models/Guide.go", model);
    let relative = "server/controllers/GuideController.go";
    write(
        root.path(),
        relative,
        "// @RequestRoute(\"/guide\")\n// @Public\ntype GuideController struct {\n}\n\n// @GetRoute\nfunc (controller *GuideController) GetGuide() any {\n\treturn Guide{Title: \"Guide\"}\n}\n",
    );
    let failure = HandlerSource::discover(root.path(), relative).expect_err("compile error");
    Some(failure.to_string())
}

fn go_is_required_in_ci() {
    let required = cfg!(unix) && std::env::var_os("CI").is_some();
    assert!(
        available("go") || !required,
        "go is required for the Go model tests in CI"
    );
}

#[tokio::test]
async fn a_typescript_middleware_imports_a_model_type() {
    if !available("bun") {
        eprintln!("skipping: bun is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/models/Badge.ts",
        "export interface Badge {\n  name: string\n  level: number\n}\n",
    );
    write(
        root.path(),
        "middleware.ts",
        "import type { Badge } from '/models/Badge.ts'\n\nexport class Middleware {\n  static before(request: Request): Response {\n    const badge: Badge = { name: 'bronze', level: 1 }\n    return new Response(null, { status: 204, headers: { 'x-badge': `${badge.name}-${request.method}` } })\n  }\n}\n",
    );
    let source = HandlerSource::discover(root.path(), "middleware.ts").expect("middleware");
    let mut options = HandlerSupervisorOptions::default();
    options.runtimes.javascript = "bun".into();
    let supervisor = HandlerSupervisor::new(options).expect("supervisor");
    let mut request = HandlerRequest::route("middleware", "/", HttpMethod::Get);
    request.operation = String::from("middleware.before");
    let response = supervisor
        .invoke(&source, &request, &HandlerCancellation::default())
        .await
        .expect("invocation");
    assert!(response.error.is_none(), "{:?}", response.error);
    assert_eq!(response.status, 204, "{response:?}");
    assert_eq!(
        response.headers.get("x-badge"),
        Some(&vec![String::from("bronze-GET")])
    );
}

#[tokio::test]
async fn a_python_test_unit_uses_a_model() {
    if !available("python3") {
        eprintln!("skipping: python3 is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, source) in Project::PYTHON.files {
        if !relative.starts_with("server/controllers/") {
            write(root.path(), relative, source);
        }
    }
    write(
        root.path(),
        "server/setups/CatalogSetup.py",
        "@setup()\nclass CatalogSetup:\n    @staticmethod\n    def configure(execution):\n        return {}\n",
    );
    let relative = "server/tests/GuideServiceTest.py";
    write(
        root.path(),
        relative,
        "from models import Guide, Kind\n\n\n@Test(CatalogSetup, {\"partition\":\"tests\"})\nclass GuideServiceTest:\n    async def test_builds_a_guide(self):\n        guide = Guide('Guide', 3, Kind.SHORT)\n        Assert.is_true(guide.pages == 3 and guide.kind is Kind.SHORT, 'the guide keeps its fields')\n",
    );
    let source = HandlerSource::discover(root.path(), relative).expect("test unit discovery");
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let response = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("test_unit", "/", HttpMethod::Post),
            &HandlerCancellation::default(),
        )
        .await
        .expect("invocation");
    assert!(response.error.is_none(), "{:?}", response.error);
    let body = response.body.expect("body").data;
    let report: serde_json::Value = serde_json::from_str(&body).expect("JSON report");
    assert_eq!(report["cases"][0]["passed"], true, "{body}");
}
