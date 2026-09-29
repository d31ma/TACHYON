#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;
use std::time::Duration;
use tachyon_contracts::{HandlerBody, HandlerBodyEncoding, HandlerRequest, HttpMethod};
use tachyon_core::{
    ApplicationConfig, EnvironmentPolicy, HandlerCancellation, HandlerSource, HandlerSupervisor,
    HandlerSupervisorOptions,
};

struct Programs;

impl Programs {
    const PYTHON: &str = r#"import json
import os
import subprocess
import sys
import time

call = json.load(sys.stdin)
mode = call["arguments"][0]
if mode == "echo":
    print(json.dumps({"method": call["method"], "arguments": call["arguments"], "language": "python"}))
elif mode == "fail":
    sys.stderr.write("secret-canary stack trace")
    sys.exit(3)
elif mode == "garbage":
    print("secret-canary is not JSON")
elif mode == "large":
    print(json.dumps("x" * (1024 * 1024 + 16)))
elif mode == "noisy":
    sys.stderr.write("x" * (1024 * 1024))
    print(json.dumps({"accepted": True}))
elif mode == "environment":
    resources = [name for name in os.environ if name.upper().startswith("YON_")]
    print(json.dumps({"yon": sorted(resources), "ordinary": os.environ.get(call["arguments"][1])}))
elif mode == "window":
    started = time.time()
    time.sleep(1.5)
    with open(os.path.join(call["arguments"][1], str(os.getpid())), "w") as target:
        target.write(json.dumps([started, time.time()]))
    print(json.dumps(True))
elif mode == "hang":
    child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"])
    with open(call["arguments"][1], "w") as target:
        target.write(str(child.pid))
    time.sleep(30)
"#;

    const NODE: &str = r"let text = ''
process.stdin.on('data', (chunk) => { text += chunk })
process.stdin.on('end', () => {
  const call = JSON.parse(text)
  process.stdout.write(JSON.stringify({ method: call.method, arguments: call.arguments, language: 'node' }))
})
";
}

struct Callers;

impl Callers {
    const JAVASCRIPT_CLIENT: &str = r#"@HTTPClient
class ReportClient {
  @Gateway({"command":["python3","server/helpers/report.py"]})
  async summarize(mode, detail) {}

  @Gateway({"command":["node","server/helpers/report.mjs"]})
  async translate(mode, detail) {}

  @Gateway({"command":["echo","1"]})
  async ping() {}
}
"#;

    const JAVASCRIPT_CONTROLLER: &str = r#"@RequestRoute("/report")
@Public
class ReportController {
  @Needs
  reportClient;

  @PostRoute
  async postReport(request) {
    const [mode, detail] = JSON.parse(await request.text())
    try {
      if (mode === 'node') return await this.reportClient.translate('echo', detail)
      if (mode === 'ping') return await this.pingMany(detail)
      if (mode === 'parallel') return await Promise.all([0, 1, 2, 3, 4].map(() => this.reportClient.summarize('window', detail)))
      if (mode === 'unknown') return await YonGateway.call('ReportClient.missing', [])
      return await this.reportClient.summarize(mode, detail)
    } catch (error) {
      return { code: error.code, message: error.message, name: error.name }
    }
  }

  async pingMany(count) {
    for (let index = 0; index < count; index += 1) await this.reportClient.ping()
    return { calls: count }
  }
}
"#;

    const TYPESCRIPT_CLIENT: &str = r#"@HTTPClient
class ReportClient {
  @Gateway({"command":["python3","server/helpers/report.py"]})
  async summarize(mode: BehaviorJsonValue, detail: BehaviorJsonValue): Promise<BehaviorJsonValue> {}
}
"#;

    const TYPESCRIPT_CONTROLLER: &str = r#"@RequestRoute("/report")
@Public
class ReportController {
  @Needs
  reportClient: ReportClient;

  @PostRoute
  async postReport(request: Request) {
    const [mode, detail] = JSON.parse(await request.text())
    try {
      return await this.reportClient.summarize(mode, detail)
    } catch (error) {
      return { code: (error as { code: string }).code }
    }
  }
}
"#;

    const PHP_CLIENT: &str = r#"<?php
#[HTTPClient]
interface ReportClient {
    #[Gateway({"command":["python3","server/helpers/report.py"]})]
    public function summarize($mode, $detail);
}
"#;

    const PHP_CONTROLLER: &str = r#"<?php
#[RequestRoute("/report")]
#[Public]
class ReportController {
    #[Needs]
    public ReportClient $reportClient;

    #[PostRoute]
    public function postReport() {
        [$mode, $detail] = json_decode(file_get_contents('php://input'), true);
        try {
            return $this->reportClient->summarize($mode, $detail);
        } catch (BehaviorError $error) {
            return ['code' => $error->behaviorCode, 'message' => $error->getMessage()];
        }
    }
}
"#;

    const JAVA_CLIENT: &str = r#"@HTTPClient
interface ReportClient {
    @Gateway({"command":["python3","server/helpers/report.py"]})
    Object summarize(Object mode, Object detail);
}
"#;

    const JAVA_CONTROLLER: &str = r#"@RequestRoute("/report")
@Public
@NeedsAll()
class ReportController {
    ReportClient reportClient;
    @PostRoute
    Object postReport(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        String text = new String(exchange.getRequestBody().readAllBytes(), java.nio.charset.StandardCharsets.UTF_8);
        java.util.List<?> input = (java.util.List<?>) Json.parse(text);
        try {
            return reportClient.summarize(input.get(0), input.get(1));
        } catch (Behaviors.Failure error) {
            return java.util.Map.of("code", error.code, "message", error.getMessage());
        }
    }
}
"#;

    const KOTLIN_CLIENT: &str = r#"@HTTPClient
interface ReportClient {
    @Gateway({"command":["python3","server/helpers/report.py"]})
    fun summarize(mode: Any?, detail: Any?): Any?
}
"#;

    const KOTLIN_CONTROLLER: &str = r#"@RequestRoute("/report")
@Public
@Needs(ReportClient::class)
class ReportController(val reportClient: ReportClient) {
    @PostRoute
    fun postReport(exchange: com.sun.net.httpserver.HttpExchange): Any? {
        val input = Json.parse(String(exchange.requestBody.readAllBytes())) as List<*>
        return try {
            reportClient.summarize(input[0], input[1])
        } catch (error: Behaviors.Failure) {
            mapOf("code" to error.code, "message" to error.message)
        }
    }
}
"#;

    const CSHARP_CLIENT: &str = r#"[HTTPClient]
interface ReportClient {
    [Gateway({"command":["python3","server/helpers/report.py"]})]
    BehaviorJSON summarize(BehaviorJSON mode, BehaviorJSON detail);
}
"#;

    const CSHARP_CONTROLLER: &str = r#"[RequestRoute("/report")]
[Public]
[Needs(typeof(ReportClient))]
class ReportController {
    readonly ReportClient reportClient;
    public ReportController(ReportClient reportClient) { this.reportClient = reportClient; }
    [PostRoute]
    public object PostReport(HttpRequestMessage request) {
        var input = BehaviorJSON.Parse(request.Content.ReadAsStringAsync().Result).Items;
        try {
            return reportClient.summarize(input[0], input[1]);
        } catch (BehaviorError error) {
            return new { code = error.Code, message = error.Message };
        }
    }
}
"#;

    const RUST_CLIENT: &str = r#"#[HTTPClient]
trait ReportClient {
    #[Gateway({"command":["python3","server/helpers/report.py"]})]
    fn summarize(&self, mode: BehaviorValue, detail: BehaviorValue) -> BehaviorResult<BehaviorValue>;
}
"#;

    const RUST_CONTROLLER: &str = r#"#[RequestRoute("/report")]
#[Public]
#[yon::needs(ReportClient)]
struct ReportController { report_client: std::sync::Arc<ReportClient> }
impl ReportController {
    fn new(report_client: std::sync::Arc<ReportClient>) -> Self { Self { report_client } }
    #[PostRoute]
    fn post_report(&self, request: &http::Request<Vec<u8>>) -> Json {
        let text = String::from_utf8_lossy(request.body()).into_owned();
        let input: Vec<BehaviorValue> = Json::parse(&text).expect("JSON body").items().iter().cloned().map(BehaviorValue::from).collect();
        match self.report_client.summarize(input[0].clone(), input[1].clone()) {
            Ok(value) => Json::parse(&value.to_json(1_048_576).expect("JSON answer")).expect("JSON value"),
            Err(error) => Json::map([("code", error.code), ("message", error.message)]),
        }
    }
}
"#;

    const DART_CLIENT: &str = r#"@HTTPClient
abstract interface class ReportClient {
  @Gateway({"command":["python3","server/helpers/report.py"]})
  Future<Object?> summarize(Object? mode, Object? detail);
}
"#;

    const DART_CONTROLLER: &str = r#"import 'dart:convert';
import 'package:shelf/shelf.dart';

@RequestRoute("/report")
@Public
class ReportController {
  @Needs
  final ReportClient reportClient;

  @PostRoute
  Future<Object?> postReport(Request request) async {
    final input = jsonDecode(await request.readAsString()) as List;
    try {
      return await reportClient.summarize(input[0], input[1]);
    } on BehaviorError catch (error) {
      return {'code': error.code, 'message': error.message};
    }
  }
}
"#;

    const GO_CLIENT: &str = r#"// @HTTPClient
type ReportClient interface {
	// @Gateway({"command":["python3","server/helpers/report.py"]})
	Summarize(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error)
}
"#;

    const GO_CONTROLLER: &str = r#"import (
	"encoding/json"
	"io"
	"net/http"
)

// @RequestRoute("/report")
// @Public
type ReportController struct {
	// @Needs
	reportClient *ReportClient
}

// @PostRoute
func (controller *ReportController) PostReport(request *http.Request) any {
	var input []any
	body, _ := io.ReadAll(request.Body)
	if err := json.Unmarshal(body, &input); err != nil {
		return map[string]any{"code": "invalid"}
	}
	value, err := controller.reportClient.Summarize(input[0], input[1])
	if err != nil {
		return map[string]any{"code": BehaviorErrorCode(err), "message": err.(*BehaviorError).Message}
	}
	return value
}
"#;

    const PYTHON_CLIENT: &str = r#"@http_client
class ReportClient(Protocol):
    @gateway({"command":["python3","server/helpers/report.py"]})
    def summarize(self, mode, detail): ...

    @gateway({"command":["node","server/helpers/report.mjs"]})
    async def translate(self, mode, detail): ...
"#;

    const PYTHON_CONTROLLER: &str = r#"import json


@request_route("/report")
@public
class ReportController:
    report_client: Annotated[ReportClient, needs]

    @post_route
    async def post_report(self, request):
        mode, detail = json.loads(request.data)
        try:
            if mode == "node":
                return await self.report_client.translate("echo", detail)
            return self.report_client.summarize(mode, detail)
        except BehaviorError as error:
            return {"code": error.code, "message": str(error)}
"#;
}

fn available(program: &str) -> bool {
    let version = if program == "go" {
        "version"
    } else {
        "--version"
    };
    StdCommand::new(program)
        .arg(version)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn usable(program: &str) -> bool {
    if available(program) {
        return true;
    }
    let required = cfg!(unix) && std::env::var_os("CI").is_some();
    assert!(
        !required,
        "{program} is required for the gateway tests in CI"
    );
    eprintln!("skipping: {program} is unavailable");
    false
}

fn kotlin_available() -> bool {
    StdCommand::new("kotlinc")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, contents).expect("file");
}

struct Project {
    root: tempfile::TempDir,
    source: HandlerSource,
}

fn project(extension: &str, client: &str, controller: &str) -> Project {
    let root = tempfile::tempdir().expect("project");
    write(root.path(), "server/helpers/report.py", Programs::PYTHON);
    write(root.path(), "server/helpers/report.mjs", Programs::NODE);
    write(
        root.path(),
        "server/application.json",
        r#"{"datasource":{"driver":"sqlite","url":"sqlite::memory:"},"catalog":{"label":"secret-canary"}}"#,
    );
    write(
        root.path(),
        &format!("server/clients/ReportClient.{extension}"),
        client,
    );
    let entry = format!("server/controllers/ReportController.{extension}");
    write(root.path(), &entry, controller);
    let source = HandlerSource::discover(root.path(), &entry).expect("gateway caller");
    Project { root, source }
}

fn supervisor(project: &Project, options: HandlerSupervisorOptions) -> HandlerSupervisor {
    let application = ApplicationConfig::load(project.root.path()).expect("application");
    HandlerSupervisor::new(HandlerSupervisorOptions {
        application,
        ..options
    })
    .expect("supervisor")
}

fn typescript_options() -> HandlerSupervisorOptions {
    let mut options = HandlerSupervisorOptions::default();
    options.runtimes.javascript = "bun".into();
    options
}

async fn call(
    project: &Project,
    supervisor: &HandlerSupervisor,
    arguments: &serde_json::Value,
) -> serde_json::Value {
    let mut request = HandlerRequest::route("gateway_call", "/report", HttpMethod::Post);
    request.headers.insert(
        String::from("authorization"),
        vec![String::from("Bearer caller-secret")],
    );
    request.body = Some(HandlerBody {
        encoding: HandlerBodyEncoding::Utf8,
        data: arguments.to_string(),
    });
    let response = supervisor
        .invoke(&project.source, &request, &HandlerCancellation::default())
        .await
        .expect("caller response");
    let data = response.body.expect("body").data;
    assert!(!data.contains("secret-canary"), "{data}");
    serde_json::from_str(&data).expect("JSON body")
}

fn callers() -> Vec<(&'static str, Project, HandlerSupervisorOptions)> {
    let mut callers = Vec::new();
    if usable("node") {
        let project = project(
            "js",
            Callers::JAVASCRIPT_CLIENT,
            Callers::JAVASCRIPT_CONTROLLER,
        );
        callers.push(("js", project, HandlerSupervisorOptions::default()));
    }
    if usable("bun") {
        let project = project(
            "ts",
            Callers::TYPESCRIPT_CLIENT,
            Callers::TYPESCRIPT_CONTROLLER,
        );
        callers.push(("ts", project, typescript_options()));
    }
    let python = project("py", Callers::PYTHON_CLIENT, Callers::PYTHON_CONTROLLER);
    callers.push(("py", python, HandlerSupervisorOptions::default()));
    if available("php-cgi") {
        let php = project("php", Callers::PHP_CLIENT, Callers::PHP_CONTROLLER);
        callers.push(("php", php, HandlerSupervisorOptions::default()));
    }
    callers
}

#[tokio::test]
async fn callers_in_each_dynamic_language_reach_a_command_with_json_arguments() {
    if !usable("python3") {
        return;
    }
    for (extension, project, options) in callers() {
        let supervisor = supervisor(&project, options);
        let detail = serde_json::json!({"period": "2026-Q3", "totals": [1, 2.5, null, true]});
        let answer = call(&project, &supervisor, &serde_json::json!(["echo", detail])).await;
        assert_eq!(answer["method"], "summarize", "{extension}: {answer}");
        assert_eq!(
            answer["arguments"],
            serde_json::json!(["echo", detail]),
            "{extension}"
        );
        assert_eq!(answer["language"], "python", "{extension}");
    }
}

fn jvm_callers() -> Vec<(&'static str, Project)> {
    let mut callers = Vec::new();
    if available("javac") {
        let java = project("java", Callers::JAVA_CLIENT, Callers::JAVA_CONTROLLER);
        callers.push(("java", java));
    }
    if kotlin_available() {
        let kotlin = project("kt", Callers::KOTLIN_CLIENT, Callers::KOTLIN_CONTROLLER);
        callers.push(("kt", kotlin));
    }
    callers
}

#[tokio::test]
async fn jvm_callers_reach_a_command_and_see_only_fixed_failures() {
    if !usable("python3") {
        return;
    }
    for (extension, project) in jvm_callers() {
        let supervisor = supervisor(&project, HandlerSupervisorOptions::default());
        let detail = serde_json::json!({"period": "2026-Q3", "totals": [1, 2.5, null, true]});
        let answer = call(&project, &supervisor, &serde_json::json!(["echo", detail])).await;
        assert_eq!(answer["method"], "summarize", "{extension}: {answer}");
        assert_eq!(
            answer["arguments"],
            serde_json::json!(["echo", detail]),
            "{extension}"
        );
        for mode in ["fail", "garbage", "large"] {
            let failed = call(&project, &supervisor, &serde_json::json!([mode, null])).await;
            assert_eq!(failed["code"], "adapter", "{extension} {mode}: {failed}");
            assert_eq!(
                failed["message"], "Gateway call failed.",
                "{extension} {mode}"
            );
        }
    }
}

fn native_callers() -> Vec<(&'static str, Project)> {
    let mut callers = Vec::new();
    if available("dotnet") {
        let csharp = project("cs", Callers::CSHARP_CLIENT, Callers::CSHARP_CONTROLLER);
        callers.push(("cs", csharp));
    }
    if available("rustc") {
        let rust = project("rs", Callers::RUST_CLIENT, Callers::RUST_CONTROLLER);
        callers.push(("rs", rust));
    }
    if available("dart") {
        let dart = project("dart", Callers::DART_CLIENT, Callers::DART_CONTROLLER);
        callers.push(("dart", dart));
    }
    if usable("go") {
        let go = project("go", Callers::GO_CLIENT, Callers::GO_CONTROLLER);
        callers.push(("go", go));
    }
    callers
}

#[tokio::test]
async fn native_callers_reach_a_command_and_see_only_fixed_failures() {
    if !usable("python3") {
        return;
    }
    for (extension, project) in native_callers() {
        let supervisor = supervisor(&project, HandlerSupervisorOptions::default());
        let detail = serde_json::json!({"period": "2026-Q3", "totals": [1, 2.5, null, true]});
        let answer = call(&project, &supervisor, &serde_json::json!(["echo", detail])).await;
        let method = if extension == "go" {
            "Summarize"
        } else {
            "summarize"
        };
        assert_eq!(answer["method"], method, "{extension}: {answer}");
        assert_eq!(
            answer["arguments"],
            serde_json::json!(["echo", detail]),
            "{extension}"
        );
        for mode in ["fail", "garbage", "large"] {
            let failed = call(&project, &supervisor, &serde_json::json!([mode, null])).await;
            assert_eq!(failed["code"], "adapter", "{extension} {mode}: {failed}");
            assert_eq!(
                failed["message"], "Gateway call failed.",
                "{extension} {mode}"
            );
        }
    }
}

#[tokio::test]
async fn a_second_program_answers_under_its_own_method_name() {
    if !usable("python3") || !usable("node") {
        return;
    }
    for (extension, client, controller) in [
        (
            "js",
            Callers::JAVASCRIPT_CLIENT,
            Callers::JAVASCRIPT_CONTROLLER,
        ),
        ("py", Callers::PYTHON_CLIENT, Callers::PYTHON_CONTROLLER),
    ] {
        let project = project(extension, client, controller);
        let supervisor = supervisor(&project, HandlerSupervisorOptions::default());
        let answer = call(&project, &supervisor, &serde_json::json!(["node", 7])).await;
        assert_eq!(answer["method"], "translate", "{extension}: {answer}");
        assert_eq!(answer["language"], "node", "{extension}");
        assert_eq!(
            answer["arguments"],
            serde_json::json!(["echo", 7]),
            "{extension}"
        );
    }
}

#[tokio::test]
async fn failing_programs_reach_the_caller_only_as_a_fixed_adapter_error() {
    if !usable("python3") {
        return;
    }
    for (extension, project, options) in callers() {
        let supervisor = supervisor(&project, options);
        for mode in ["fail", "garbage", "large", "noisy"] {
            let answer = call(&project, &supervisor, &serde_json::json!([mode, null])).await;
            assert_eq!(answer["code"], "adapter", "{extension} {mode}: {answer}");
            if extension != "ts" {
                assert_eq!(
                    answer["message"], "Gateway call failed.",
                    "{extension} {mode}"
                );
            }
        }
    }
}

#[tokio::test]
async fn a_command_receives_only_the_baseline_and_allowlisted_environment() {
    if !usable("python3") {
        return;
    }
    let (name, value) = ["HOME", "USERPROFILE", "CARGO_HOME", "CI"]
        .iter()
        .find_map(|name| std::env::var(name).ok().map(|value| (*name, value)))
        .expect("one ordinary environment variable");
    for (extension, project, options) in callers() {
        let supervisor = supervisor(
            &project,
            HandlerSupervisorOptions {
                environment: EnvironmentPolicy::from_names([
                    name,
                    "YON_APPLICATION_JSON",
                    "yon_application_json",
                    "YON_RESOURCE_TOKEN",
                    "Yon_Resource_Future",
                    "YON_HANDLER_PROTOCOL",
                ])
                .expect("allowlist"),
                ..options
            },
        );
        let answer = call(
            &project,
            &supervisor,
            &serde_json::json!(["environment", name]),
        )
        .await;
        assert_eq!(
            answer["yon"],
            serde_json::json!([]),
            "{extension}: {answer}"
        );
        assert_eq!(answer["ordinary"], value, "{extension}: {answer}");
    }
}

#[test]
fn explicit_allowlisting_never_transfers_private_environment_values() {
    let result = StdCommand::new(std::env::current_exe().expect("test binary"))
        .args([
            "--exact",
            "gateway_commands::a_command_receives_only_the_baseline_and_allowlisted_environment",
            "--nocapture",
        ])
        .env("YON_APPLICATION_JSON", "synthetic-private-configuration")
        .env("yon_application_json", "synthetic-private-configuration")
        .env("YON_RESOURCE_TOKEN", "synthetic-private-token")
        .env("Yon_Resource_Future", "synthetic-private-capability")
        .env("YON_HANDLER_PROTOCOL", "synthetic-private-protocol")
        .output()
        .expect("environment regression process");
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
    );
}

fn process_exists(pid: u32) -> bool {
    StdCommand::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(unix)]
#[tokio::test]
async fn a_slow_command_times_out_and_its_descendants_are_reaped() {
    if !usable("python3") {
        return;
    }
    for (extension, project, options) in callers() {
        let supervisor = supervisor(
            &project,
            HandlerSupervisorOptions {
                default_timeout: Duration::from_secs(3),
                cancellation_grace: Duration::from_millis(500),
                ..options
            },
        );
        let pid_file = project.root.path().join("descendant.pid");
        let arguments = serde_json::json!(["hang", pid_file.to_string_lossy()]);
        let started = std::time::Instant::now();
        let answer = call(&project, &supervisor, &arguments).await;
        assert_eq!(answer["code"], "timeout", "{extension}: {answer}");
        assert!(started.elapsed() < Duration::from_secs(3), "{extension}");
        let pid: u32 = fs::read_to_string(&pid_file)
            .expect("pid file")
            .trim()
            .parse()
            .expect("pid");
        let mut gone = false;
        for _attempt in 0..100 {
            if !process_exists(pid) {
                gone = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(gone, "{extension}: descendant {pid} survived");
    }
}

#[tokio::test]
async fn one_invocation_may_make_at_most_sixty_four_calls() {
    if !usable("node") {
        return;
    }
    let project = project(
        "js",
        Callers::JAVASCRIPT_CLIENT,
        Callers::JAVASCRIPT_CONTROLLER,
    );
    let supervisor = supervisor(&project, HandlerSupervisorOptions::default());
    let allowed = call(&project, &supervisor, &serde_json::json!(["ping", 64])).await;
    assert_eq!(allowed["calls"], 64, "{allowed}");
    let refused = call(&project, &supervisor, &serde_json::json!(["ping", 65])).await;
    assert_eq!(refused["code"], "capacity", "{refused}");
}

#[tokio::test]
async fn an_undeclared_key_is_invalid() {
    if !usable("node") {
        return;
    }
    let project = project(
        "js",
        Callers::JAVASCRIPT_CLIENT,
        Callers::JAVASCRIPT_CONTROLLER,
    );
    let supervisor = supervisor(&project, HandlerSupervisorOptions::default());
    let answer = call(&project, &supervisor, &serde_json::json!(["unknown", null])).await;
    assert_eq!(answer, serde_json::json!({"error": "invalid"}));
}

#[test]
fn command_files_must_be_captured_helpers() {
    for (command, expected) in [
        (r#"["python3","server/helpers/missing.py"]"#, "TY1312"),
        (
            r#"["python3","server/helpers/../application.json"]"#,
            "TY1312",
        ),
        (
            r#"["python3","server/controllers/ReportController.js"]"#,
            "TY1312",
        ),
        (r#"["./report"]"#, "TY1308"),
    ] {
        let client = format!(
            "@HTTPClient\nclass ReportClient {{\n  @Gateway({{\"command\":{command}}})\n  async summarize() {{}}\n}}\n"
        );
        let root = tempfile::tempdir().expect("project");
        write(root.path(), "server/clients/ReportClient.js", &client);
        write(
            root.path(),
            "server/controllers/ReportController.js",
            "@RequestRoute(\"/report\")\n@Public\nclass ReportController {\n  @GetRoute\n  getReport() { return {} }\n}\n",
        );
        let failure =
            HandlerSource::discover(root.path(), "server/controllers/ReportController.js")
                .expect_err("refused command");
        assert!(
            failure.to_string().contains(expected),
            "{command}: {failure}"
        );
    }
}

fn most_calls_at_once(directory: &Path) -> usize {
    let mut windows = Vec::new();
    for entry in fs::read_dir(directory).expect("windows") {
        let text = fs::read_to_string(entry.expect("window").path()).expect("window file");
        let window: (f64, f64) = serde_json::from_str(&text).expect("window JSON");
        windows.push(window);
    }
    assert_eq!(windows.len(), 5, "{windows:?}");
    let mut most = 0;
    for (start, _) in &windows {
        let mut running = 0;
        for (other_start, other_end) in &windows {
            if other_start <= start && start < other_end {
                running += 1;
            }
        }
        most = most.max(running);
    }
    most
}

#[tokio::test]
async fn one_invocation_runs_at_most_four_calls_at_once() {
    if !usable("node") || !usable("python3") {
        return;
    }
    let project = project(
        "js",
        Callers::JAVASCRIPT_CLIENT,
        Callers::JAVASCRIPT_CONTROLLER,
    );
    let supervisor = supervisor(&project, HandlerSupervisorOptions::default());
    let windows = project.root.path().join("windows");
    fs::create_dir_all(&windows).expect("windows directory");
    let answer = call(
        &project,
        &supervisor,
        &serde_json::json!(["parallel", windows.to_string_lossy()]),
    )
    .await;
    assert_eq!(answer, serde_json::json!([true, true, true, true, true]));
    assert_eq!(most_calls_at_once(&windows), 4);
}

#[tokio::test]
async fn commands_run_while_the_handler_pool_is_full() {
    if !usable("node") || !usable("python3") {
        return;
    }
    let project = project(
        "js",
        Callers::JAVASCRIPT_CLIENT,
        Callers::JAVASCRIPT_CONTROLLER,
    );
    let options = HandlerSupervisorOptions {
        max_concurrency: 1,
        default_timeout: Duration::from_secs(10),
        ..HandlerSupervisorOptions::default()
    };
    let supervisor = supervisor(&project, options);
    let answer = call(&project, &supervisor, &serde_json::json!(["echo", 1])).await;
    assert_eq!(answer["method"], "summarize", "{answer}");
}
