#![allow(clippy::expect_used)]

use std::path::Path;
use std::process::Command;

struct GatewayProject;

impl GatewayProject {
    const FILES: [(&str, &str); 5] = [
        ("server/helpers/report.py", "print(1)\n"),
        (
            "server/remotes/ReportClient.js",
            "@HTTPClient\nclass ReportClient {\n  @Gateway({\"command\":[\"tachyon-missing-gateway-program\",\"server/helpers/report.py\"]})\n  async summarize(period) {}\n}\n",
        ),
        (
            "server/services/ReportService.js",
            "@Service()\nclass ReportService {\n  @Needs()\n  reportClient;\n  total() { return this.reportClient.summarize('q3'); }\n}\n",
        ),
        (
            "server/tests/ReportTestConfiguration.js",
            "@TestSetup()\nclass ReportTestConfiguration {\n  static configure() { return {}; }\n}\n",
        ),
        (
            "server/tests/ReportServiceTest.js",
            "@Test(ReportTestConfiguration)\nexport default class ReportServiceTest {\n  async testTotal() {\n    Assert.equals(1, await this.construct(ReportService).total());\n  }\n}\n",
        ),
    ];

    fn write(root: &Path) {
        for (relative, text) in Self::FILES {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, text).expect("file");
        }
    }
}

#[test]
fn test_preflight_requires_the_programs_that_gateways_name() {
    let project = tempfile::tempdir().expect("project");
    GatewayProject::write(project.path());
    let output = Command::new(env!("CARGO_BIN_EXE_ty"))
        .arg("test")
        .arg(project.path())
        .output()
        .expect("ty test");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("TY2112") && stderr.contains("tachyon-missing-gateway-program"),
        "{stderr}{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

struct GoProject;

impl GoProject {
    const FILES: [(&str, &str); 3] = [
        ("client/pages/tac.html", "<main><h1>Go</h1></main>\n"),
        (
            "server/controllers/HelloController.go",
            "#[RequestRoute(\"/hello\")]\n#[Public]\ntype HelloController struct{}\n\n#[GetRoute]\nfunc (controller *HelloController) Get() any {\n\treturn map[string]any{}\n}\n",
        ),
        (
            "middleware.go",
            "import \"net/http\"\n\ntype Middleware struct{}\n\nfunc (Middleware) After(request *http.Request) *http.Response {\n\treturn nil\n}\n",
        ),
    ];

    fn write(root: &Path) {
        for (relative, text) in Self::FILES {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, text).expect("file");
        }
    }
}

fn run_with_an_empty_path(project: &Path, arguments: &[&str]) -> (bool, String) {
    let empty_path = tempfile::tempdir().expect("empty PATH directory");
    let output = Command::new(env!("CARGO_BIN_EXE_ty"))
        .args(arguments)
        .arg(project)
        .env("PATH", empty_path.path())
        .output()
        .expect("ty");
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), printed)
}

#[test]
fn go_handlers_without_go_fail_with_the_missing_toolchain_error() {
    let project = tempfile::tempdir().expect("project");
    GoProject::write(project.path());
    let (served, printed) =
        run_with_an_empty_path(project.path(), &["serve", "--port", "0", "--no-watch"]);
    assert!(!served, "{printed}");
    assert!(
        printed.contains("TY2112")
            && printed.contains("Required Yon Go compiler 'go' was not found."),
        "{printed}"
    );
    assert!(
        printed.contains("Install Go 1.27 or newer before starting Tachyon."),
        "{printed}"
    );
    let (doctor_ready, doctor) = run_with_an_empty_path(project.path(), &["doctor"]);
    assert!(!doctor_ready, "{doctor}");
    assert!(
        doctor.contains("missing   Yon Go compiler 'go'  go  not on PATH"),
        "{doctor}"
    );
}

#[test]
fn doctor_succeeds_when_only_a_native_target_toolchain_is_missing() {
    let project = tempfile::tempdir().expect("project");
    let services = project.path().join("client/services");
    std::fs::create_dir_all(&services).expect("services");
    std::fs::write(
        services.join("CatalogService.kt"),
        "@Service\nclass CatalogService\n",
    )
    .expect("kotlin");
    std::fs::write(
        services.join("CatalogService.js"),
        "@Service\nexport default class CatalogService {}\n",
    )
    .expect("script");
    let (doctor_ready, doctor) = run_with_an_empty_path(project.path(), &["doctor"]);
    assert!(doctor_ready, "{doctor}");
    assert!(
        doctor.contains("info      Kotlin services for android"),
        "{doctor}"
    );
}

struct CompiledRoute;

impl CompiledRoute {
    const ROUTES: [(&str, &str, &str, &str); 4] = [
        (
            "server/controllers/HelloController.rs",
            "#[RequestRoute(\"/hello\")]\n#[Public]\nstruct HelloController {}\nimpl HelloController {\n    fn new() -> Self { Self {} }\n    #[GetRoute]\n    fn get(&self) -> String {\n        String::from(\"ok\")\n    }\n}\n",
            "Required Yon Rust build tool 'cargo' was not found.",
            "Install Rust before starting Tachyon.",
        ),
        (
            "server/controllers/HelloController.kt",
            "@RequestRoute(\"/hello\")\n@Public\nclass HelloController {\n    @GetRoute\n    fun get(): Any? = mapOf<String, Any?>()\n}\n",
            "Required Yon Kotlin compiler 'kotlinc' was not found.",
            "Install the Kotlin compiler before starting Tachyon.",
        ),
        (
            "server/controllers/HelloController.dart",
            "@RequestRoute(\"/hello\")\n@Public\nclass HelloController {\n  @GetRoute\n  Object? get() => {'ok': true};\n}\n",
            "Required Yon Dart compiler 'dart' was not found.",
            "Install the Dart SDK before starting Tachyon.",
        ),
        (
            "server/controllers/HelloController.cs",
            "[RequestRoute(\"/hello\")]\n[Public]\nsealed class HelloController {\n    [GetRoute]\n    public object Get() => new { };\n}\n",
            "Required Yon C# runtime 'dotnet' was not found.",
            "Install the .NET SDK and Microsoft.NETCore.App runtime.",
        ),
    ];

    fn write(root: &Path, route: &str, source: &str) {
        let page = root.join("client/pages/tac.html");
        std::fs::create_dir_all(page.parent().expect("parent")).expect("directory");
        std::fs::write(page, "<main><h1>Compiled</h1></main>\n").expect("page");
        let path = root.join(route);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("route");
    }
}

#[test]
fn compiled_handlers_without_their_compiler_fail_with_the_missing_toolchain_error() {
    for (route, source, message, help) in CompiledRoute::ROUTES {
        let project = tempfile::tempdir().expect("project");
        CompiledRoute::write(project.path(), route, source);
        let (served, printed) =
            run_with_an_empty_path(project.path(), &["serve", "--port", "0", "--no-watch"]);
        assert!(!served, "{route}: {printed}");
        assert!(printed.contains("TY2112"), "{route}: {printed}");
        assert!(printed.contains(message), "{route}: {printed}");
        assert!(printed.contains(help), "{route}: {printed}");
    }
}
