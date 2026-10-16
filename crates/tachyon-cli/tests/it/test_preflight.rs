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
    const ROUTES: [(&str, &str, &str, &str); 5] = [
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
        (
            "server/controllers/HelloController.hpp",
            "class [[RequestRoute(\"/hello\")]] [[Public]] HelloController {\npublic:\n    [[GetRoute]]\n    bool getHello() { return true; }\n};\n",
            "Required Yon C++ compiler 'clang++' was not found.",
            "Install clang++ with C++20 support",
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

#[test]
fn doctor_names_clang_for_a_cpp_controller() {
    let project = tempfile::tempdir().expect("project");
    let (route, source, _, _) = CompiledRoute::ROUTES[4];
    CompiledRoute::write(project.path(), route, source);
    let (doctor_ready, doctor) = run_with_an_empty_path(project.path(), &["doctor"]);
    assert!(!doctor_ready, "{doctor}");
    assert!(
        doctor.contains("missing   Yon C++ compiler 'clang++'  clang++  not on PATH"),
        "{doctor}"
    );
}

#[cfg(unix)]
#[test]
fn javascript_handlers_run_with_bun_when_only_bun_is_on_the_path() {
    use std::os::unix::fs::PermissionsExt as _;
    let paths = std::env::var_os("PATH").unwrap_or_default();
    let Some(node) = std::env::split_paths(&paths)
        .map(|folder| folder.join("node"))
        .find(|candidate| candidate.is_file())
    else {
        return;
    };
    let only_bun = tempfile::tempdir().expect("PATH with only bun");
    let bun = only_bun.path().join("bun");
    std::fs::write(
        &bun,
        format!("#!/bin/sh\nexec '{}' \"$@\"\n", node.display()),
    )
    .expect("bun");
    std::fs::set_permissions(&bun, std::fs::Permissions::from_mode(0o755)).expect("mode");
    let project = tempfile::tempdir().expect("project");
    let worker = project.path().join("server/workers/probe.js");
    std::fs::create_dir_all(worker.parent().expect("parent")).expect("directory");
    std::fs::write(
        &worker,
        "export class Handler {\n  static GET() {\n    return { ran: true }\n  }\n}\n",
    )
    .expect("worker");
    let output = Command::new(env!("CARGO_BIN_EXE_ty"))
        .args(["handler", "invoke", "server/workers/probe.js", "--project"])
        .arg(project.path())
        .args(["--route", "/probe", "--method", "GET"])
        .env("PATH", only_bun.path())
        .env_remove("YON_JAVASCRIPT_RUNTIME")
        .output()
        .expect("ty");
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{printed}");
    assert!(printed.contains("ran"), "{printed}");
}

#[test]
fn a_crashing_test_unit_fails_alone_and_the_run_still_reports_every_unit() {
    let project = tempfile::tempdir().expect("project");
    for (relative, text) in [
        (
            "server/tests/UnitTestSetup.js",
            "@TestSetup()\nclass UnitTestSetup {\n  static configure() { return {}; }\n}\n",
        ),
        (
            "server/tests/ACrashTest.js",
            "@Test(UnitTestSetup)\nexport default class ACrashTest {\n  async testExits() {\n    process.exit(3)\n  }\n}\n",
        ),
        (
            "server/tests/BOkTest.js",
            "@Test(UnitTestSetup)\nexport default class BOkTest {\n  async testPasses() {\n    Assert.equals(1, 1)\n  }\n}\n",
        ),
    ] {
        let path = project.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, text).expect("file");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_ty"))
        .arg("test")
        .arg(project.path())
        .output()
        .expect("ty test");
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.status.success(), "{printed}");
    assert!(
        printed.contains("FAIL server/tests/ACrashTest.js: TY2104"),
        "{printed}"
    );
    assert!(printed.contains("ok  BOkTest.testPasses"), "{printed}");
    assert!(printed.contains("1 passed, 1 failed."), "{printed}");
    assert!(printed.contains("TY2020"), "{printed}");
}
