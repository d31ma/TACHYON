#![allow(clippy::expect_used)]

use std::fs;
#[cfg(unix)]
use std::path::Path;
use std::process::Command as StdCommand;
#[cfg(unix)]
use std::process::Stdio;
use std::time::Duration;
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HandlerResponse, HttpMethod,
};
use tachyon_core::{
    EnvironmentPolicy, HandlerCancellation, HandlerSource, HandlerSupervisor,
    HandlerSupervisorOptions,
};

fn source(root: &tempfile::TempDir, extension: &str, contents: &str) -> HandlerSource {
    let relative = format!("server/workers/contract.{extension}");
    let path = root.path().join(&relative);
    fs::create_dir_all(path.parent().expect("handler parent")).expect("handler directory");
    let contents = String::from(contents);
    fs::write(path, contents).expect("handler source");
    HandlerSource::discover(root.path(), &relative).expect("validated source")
}

fn request(id: &str, method: HttpMethod, body: Option<&str>) -> HandlerRequest {
    let mut request = HandlerRequest::route(id, "/contract", method);
    request.headers.insert(
        String::from("x-contract"),
        vec![String::from("shared"), String::from("second")],
    );
    request.body = body.map(|data| HandlerBody {
        encoding: HandlerBodyEncoding::Utf8,
        data: String::from(data),
    });
    request
}

fn body_json(response: &HandlerResponse) -> serde_json::Value {
    let body = response.body.as_ref().expect("successful body");
    serde_json::from_str(&body.data).expect("JSON handler result")
}

fn supervisor(options: HandlerSupervisorOptions) -> HandlerSupervisor {
    HandlerSupervisor::new(options).expect("valid supervisor")
}

fn available(program: &str) -> bool {
    let version = match program {
        "kotlinc" => "-version",
        "php-cgi" => "-v",
        "go" => "version",
        _ => "--version",
    };
    StdCommand::new(program)
        .arg(version)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(unix)]
fn process_exists(pid: u32) -> bool {
    StdCommand::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(windows)]
fn process_exists(pid: u32) -> bool {
    StdCommand::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains(&format!("\"{pid}\"")))
}

async fn assert_process_gone(pid: u32) {
    for _attempt in 0..100 {
        if !process_exists(pid) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("handler descendant {pid} survived process-group cleanup");
}

fn descendant_request(id: &str, mode: &str, pid_file: &std::path::Path) -> HandlerRequest {
    let mut request = request(id, HttpMethod::Post, Some(mode));
    request.headers.insert(
        String::from("x-pid-file"),
        vec![pid_file.to_string_lossy().into_owned()],
    );
    request
}

/// Budgets for tests whose outcome must fire after the handler has spawned its
/// descendant. A deadline shorter than the interpreter's cold start ends the
/// invocation before any descendant exists, and then proves nothing about reaping.
struct DescendantDeadline;

impl DescendantDeadline {
    /// Room for the interpreter to start, run the handler, and write the pid file.
    /// Cold starts took up to 1.4 s on a loaded Windows host.
    const STARTUP: Duration = Duration::from_secs(5);
    /// How long the supervisor may take to settle and reap after the outcome fires.
    const SETTLE: Duration = Duration::from_secs(2);
}

/// Budgets for streams whose deadline must fire while the generator is running and
/// the subscriber queue is full. A deadline shorter than the interpreter's cold
/// start ends the stream before the first event, and then proves nothing about
/// backpressure.
struct BackpressureDeadline;

impl BackpressureDeadline {
    /// The stream deadline: room for the interpreter to start and fill the queue.
    const DEADLINE: Duration = DescendantDeadline::STARTUP;
    /// How long the supervisor may take to stop a backpressured stream after its
    /// deadline fires.
    const TEARDOWN: Duration = Duration::from_millis(1750);

    async fn assert_bounded_with_a_full_queue(source: &HandlerSource, id: &str, expectation: &str) {
        let supervisor = supervisor(HandlerSupervisorOptions {
            default_timeout: Self::DEADLINE,
            ..HandlerSupervisorOptions::default()
        });
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let started = std::time::Instant::now();
        let failure = supervisor
            .invoke_streaming(source, &request(id, HttpMethod::Get, None), sender)
            .await
            .expect_err(expectation);
        // The deadline fires no earlier than `started + DEADLINE`, so this
        // overestimates the time spent after it.
        let teardown = started.elapsed().saturating_sub(Self::DEADLINE);
        assert!(failure.to_string().contains("TY2110"), "{failure}");
        let first = receiver
            .try_recv()
            .expect("the generator filled the subscriber queue before the deadline");
        assert!(first.data.len() > 200 * 1024);
        assert!(
            teardown < Self::TEARDOWN,
            "stopping the stream took {teardown:?} after its deadline"
        );
    }
}

fn descendant_pid(pid_file: &std::path::Path) -> u32 {
    assert!(
        pid_file.is_file(),
        "the invocation ended before the handler spawned its descendant"
    );
    fs::read_to_string(pid_file)
        .expect("descendant pid file")
        .trim()
        .parse()
        .expect("descendant pid")
}

#[tokio::test]
async fn javascript_and_python_share_request_async_unicode_and_sideband_behavior() {
    let javascript = r"
export class Handler {
  static async POST(request) {
    console.log('sideband-only')
    await new Promise((resolve) => setTimeout(resolve, 5))
    return {
      method: request.method,
      headers: request.headers.get('x-contract').split(', '),
      body: await request.text(),
      unicode: 'héllø 🌍',
    }
  }
}
";
    let python = r#"
import asyncio

class Handler:
    @staticmethod
    async def POST(request):
        print("sideband-only")
        await asyncio.sleep(0.005)
        return {
            "method": request.get_method(),
            "headers": request.get_header("X-contract").split(", "),
            "body": request.data.decode(),
            "unicode": "héllø 🌍",
        }
"#;

    for (extension, contents) in [("js", javascript), ("py", python)] {
        let root = tempfile::tempdir().expect("project");
        let source = source(&root, extension, contents);
        let response = supervisor(HandlerSupervisorOptions::default())
            .invoke(
                &source,
                &request("shared_request", HttpMethod::Post, Some("payload")),
                &HandlerCancellation::default(),
            )
            .await
            .expect("handler response");
        assert_eq!(response.status, 200);
        assert!(response.error.is_none());
        let value = body_json(&response);
        assert_eq!(value["method"], "POST");
        assert_eq!(value["headers"][0], "shared");
        assert_eq!(value["headers"][1], "second");
        assert_eq!(value["body"], "payload");
        assert_eq!(value["unicode"], "héllø 🌍");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn interpreted_handlers_execute_owned_dependencies_after_the_project_root_is_swapped() {
    use std::os::unix::fs::symlink;

    let mut cases = vec![
        (
            "js",
            "import { Origin } from '../shared/origin.js'\nexport class Handler {\n  static GET() { return { origin: Origin.value() } }\n}\n",
            "export class Origin { static value() { return 'owned-js' } }\n",
            "import { Origin } from '../shared/origin.js'\nexport class Handler {\n  static GET() { return { origin: Origin.value() } }\n}\n",
            "export class Origin { static value() { return 'planted-js' } }\n",
            "owned-js",
        ),
        (
            "py",
            "from server.shared.origin import Origin\nclass Handler:\n    @staticmethod\n    def GET(request):\n        return {'origin': Origin.value()}\n",
            "class Origin:\n    @staticmethod\n    def value():\n        return 'owned-py'\n",
            "from server.shared.origin import Origin\nclass Handler:\n    @staticmethod\n    def GET(request):\n        return {'origin': Origin.value()}\n",
            "class Origin:\n    @staticmethod\n    def value():\n        return 'planted-py'\n",
            "owned-py",
        ),
    ];
    if available("bun") {
        cases.push((
            "ts",
            "import { Origin } from '../shared/origin.ts'\nexport class Handler {\n  static GET() { return { origin: Origin.value() } }\n}\n",
            "export class Origin { static value(): string { return 'owned-ts' } }\n",
            "import { Origin } from '../shared/origin.ts'\nexport class Handler {\n  static GET() { return { origin: Origin.value() } }\n}\n",
            "export class Origin { static value(): string { return 'planted-ts' } }\n",
            "owned-ts",
        ));
    }

    for (extension, owned, owned_dependency, planted, planted_dependency, expected) in cases {
        let workspace = tempfile::tempdir().expect("workspace");
        let project = workspace.path().join(format!("project-{extension}"));
        let relative = format!("server/workers/contract.{extension}");
        let authored = project.join(&relative);
        fs::create_dir_all(authored.parent().expect("handler parent")).expect("handler root");
        fs::write(&authored, owned).expect("owned source");
        fs::create_dir_all(project.join("server/shared")).expect("shared root");
        fs::write(
            project.join(format!("server/shared/origin.{extension}")),
            owned_dependency,
        )
        .expect("owned dependency");
        let source = HandlerSource::discover(&project, &relative).expect("snapshot source");

        let retained = workspace.path().join(format!("retained-{extension}"));
        fs::rename(&project, &retained).expect("move opened project");
        let outside = tempfile::tempdir().expect("planted project");
        let planted_source = outside.path().join(&relative);
        fs::create_dir_all(planted_source.parent().expect("planted parent")).expect("planted root");
        fs::write(&planted_source, planted).expect("planted source");
        fs::create_dir_all(outside.path().join("server/shared")).expect("planted shared root");
        fs::write(
            outside
                .path()
                .join(format!("server/shared/origin.{extension}")),
            planted_dependency,
        )
        .expect("planted dependency");
        symlink(outside.path(), &project).expect("ambient project replacement");

        let mut options = HandlerSupervisorOptions::default();
        if extension == "ts" {
            options.runtimes.javascript = "bun".into();
        }
        let response = supervisor(options)
            .invoke(
                &source,
                &request(
                    &format!("owned_snapshot_{extension}"),
                    HttpMethod::Get,
                    None,
                ),
                &HandlerCancellation::default(),
            )
            .await
            .expect("owned handler response");
        assert_eq!(response.status, 200, "{extension}: {response:?}");
        assert_eq!(body_json(&response)["origin"], expected, "{extension}");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn relay_executes_the_owned_delegate_after_the_project_root_is_swapped() {
    use std::os::unix::fs::symlink;

    let workspace = tempfile::tempdir().expect("workspace");
    let project = workspace.path().join("project-relay");
    let handler = project.join("server/workers/contract.py");
    let helper = project.join("server/helpers/origin.py");
    fs::create_dir_all(handler.parent().expect("handler parent")).expect("handler root");
    fs::create_dir_all(helper.parent().expect("helper parent")).expect("helper root");
    fs::write(
        &handler,
        "class Handler:\n    @staticmethod\n    def GET(request):\n        return relay(('python3', 'server/helpers/origin.py'), request)\n",
    )
    .expect("handler");
    fs::write(
        &helper,
        "import json\nprint(json.dumps({'status': 200, 'headers': {'content-type': ['application/json']}, 'body': json.dumps({'origin': 'owned-relay'})}))\n",
    )
    .expect("owned helper");
    let source =
        HandlerSource::discover(&project, "server/workers/contract.py").expect("snapshot source");

    let retained = workspace.path().join("retained-relay");
    fs::rename(&project, &retained).expect("move opened project");
    let outside = tempfile::tempdir().expect("planted project");
    let planted = outside.path().join("server/helpers/origin.py");
    fs::create_dir_all(planted.parent().expect("planted parent")).expect("planted root");
    fs::write(
        &planted,
        "import json\nprint(json.dumps({'status': 200, 'headers': {'content-type': ['application/json']}, 'body': json.dumps({'origin': 'planted-relay'})}))\n",
    )
    .expect("planted helper");
    symlink(outside.path(), &project).expect("replace ambient project");

    let response = supervisor(HandlerSupervisorOptions::default())
        .invoke(
            &source,
            &request("owned_relay", HttpMethod::Get, None),
            &HandlerCancellation::default(),
        )
        .await
        .expect("relay response");
    assert_eq!(body_json(&response)["origin"], "owned-relay");
}

#[cfg(unix)]
struct PreparedCases;

#[cfg(unix)]
impl PreparedCases {
    const ALL: [(&str, &str, &str, &str); 5] = [
        (
            "rs",
            "rustc",
            r#"
struct Handler;
impl Handler {
    fn GET() -> Json {
        Json::map([("kind", "rust")])
    }
}
"#,
            "rust",
        ),
        (
            "go",
            "go",
            r#"
type Handler struct{}

func (Handler) GET() any {
	return map[string]any{"kind": "go"}
}
"#,
            "go",
        ),
        (
            "cs",
            "dotnet",
            r#"
sealed class Handler {
    public static object GET() => new { kind = "csharp" };
}
"#,
            "csharp",
        ),
        (
            "fs",
            "dotnet",
            r#"
type Handler =
    static member GET() : obj = box {| kind = "fsharp" |}
"#,
            "fsharp",
        ),
        (
            "php",
            "php-cgi",
            r"<?php
final class Handler
{
    public static function GET(): array
    {
        return ['kind' => 'php'];
    }
}
",
            "php",
        ),
    ];
}

#[cfg(unix)]
#[tokio::test]
async fn prepared_handlers_ignore_an_ambient_project_root_swap_during_real_invocation() {
    use std::os::unix::fs::symlink;

    for (extension, tool, contents, expected) in PreparedCases::ALL {
        if !available(tool) {
            continue;
        }
        let root = tempfile::tempdir().expect("project");
        let prepared = source(&root, extension, contents);
        let absolute_artifacts = prepared
            .interpreter()
            .iter()
            .filter_map(|argument| {
                let candidate = argument
                    .split_once('=')
                    .map_or(argument.as_str(), |(_, value)| value);
                Path::new(candidate).is_absolute().then_some(candidate)
            })
            .collect::<Vec<_>>();
        assert!(!absolute_artifacts.is_empty(), "{extension}");
        for artifact in absolute_artifacts {
            assert!(Path::new(artifact).is_file(), "{extension}: {artifact}");
            assert!(
                !Path::new(artifact).starts_with(root.path()),
                "{extension} runtime still points into the authored project: {artifact}"
            );
        }

        let authored = root.path().to_path_buf();
        let retained = authored.with_extension(format!("retained-{extension}"));
        fs::rename(&authored, &retained).expect("move authored project");
        let outside = tempfile::tempdir().expect("planted project");
        let planted = outside
            .path()
            .join(format!("server/workers/contract.{extension}"));
        fs::create_dir_all(planted.parent().expect("planted parent")).expect("planted root");
        fs::write(&planted, "planted source must never execute").expect("planted source");
        symlink(outside.path(), &authored).expect("replace authored project");
        let response = supervisor(HandlerSupervisorOptions {
            default_timeout: Duration::from_secs(10),
            ..HandlerSupervisorOptions::default()
        })
        .invoke(
            &prepared,
            &request(&format!("root_swap_{extension}"), HttpMethod::Get, None),
            &HandlerCancellation::default(),
        )
        .await
        .expect("prepared handler must use its owned runtime workspace");
        assert_eq!(response.status, 200, "{extension}");
        assert_eq!(body_json(&response)["kind"], expected, "{extension}");
        assert_eq!(
            fs::read_to_string(&planted).expect("planted source remains"),
            "planted source must never execute",
            "{extension}"
        );
        fs::remove_file(&authored).expect("remove replacement symlink");
        fs::rename(&retained, &authored).expect("restore authored project for cleanup");
    }
}

fn go_usable() -> bool {
    if available("go") {
        return true;
    }
    let required = cfg!(unix) && std::env::var_os("CI").is_some();
    assert!(!required, "go is required for the Go handler tests in CI");
    eprintln!("skipping: go is unavailable");
    false
}

struct GoRoute;

impl GoRoute {
    const SOURCE: &str = r#"package main

import (
	"encoding/json"
	"io"
	"net/http"
	"strings"
)

type Handler struct{}

func (Handler) GET(request *http.Request) *http.Response {
	name := strings.ToUpper(request.URL.Query().Get("name"))
	body, _ := json.Marshal(map[string]string{"name": name, "header": request.Header.Get("X-Contract")})
	header := http.Header{"Content-Type": {"application/json"}, "X-Language": {"go"}}
	return &http.Response{StatusCode: http.StatusOK, Header: header, Body: io.NopCloser(strings.NewReader(string(body)))}
}

func (handler *Handler) POST(request *http.Request) any {
	var input map[string]any
	if err := json.NewDecoder(request.Body).Decode(&input); err != nil {
		return &http.Response{StatusCode: http.StatusBadRequest}
	}
	return map[string]any{"method": request.Method, "route": request.URL.Path, "echo": input["value"]}
}
"#;
}

#[tokio::test]
async fn a_go_worker_answers_its_verbs_and_405_for_the_others() {
    if !go_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let handler = source(&root, "go", GoRoute::SOURCE);
    let supervisor = supervisor(HandlerSupervisorOptions {
        default_timeout: Duration::from_secs(10),
        ..HandlerSupervisorOptions::default()
    });
    let cancellation = HandlerCancellation::default();
    let mut get = request("go_get", HttpMethod::Get, None);
    get.target = Some(String::from("/contract?name=ada"));
    let answered = supervisor
        .invoke(&handler, &get, &cancellation)
        .await
        .expect("GET");
    assert_eq!(answered.status, 200);
    assert_eq!(body_json(&answered)["name"], "ADA");
    assert_eq!(body_json(&answered)["header"], "shared");
    assert_eq!(answered.headers["x-language"], vec![String::from("go")]);

    let post = request("go_post", HttpMethod::Post, Some(r#"{"value":7}"#));
    let posted = supervisor
        .invoke(&handler, &post, &cancellation)
        .await
        .expect("POST");
    assert_eq!(posted.status, 200);
    assert_eq!(body_json(&posted)["method"], "POST");
    assert_eq!(body_json(&posted)["route"], "/contract");
    assert_eq!(body_json(&posted)["echo"], 7);

    for method in [HttpMethod::Put, HttpMethod::Delete, HttpMethod::Head] {
        let refused = supervisor
            .invoke(&handler, &request("go_other", method, None), &cancellation)
            .await
            .expect("undeclared verb");
        assert_eq!(refused.status, 405, "{method:?}");
    }
}

struct GoShapes;

impl GoShapes {
    const SOURCE: &str = r#"import "net/http"

type Handler struct{}

func (Handler) GET() any {
	return map[string]any{"ok": true}
}

func (Handler) POST(request *http.Request) (any, error) {
	return map[string]any{"method": request.Method}, nil
}

func (Handler) PUT() error {
	return nil
}

func (Handler) PATCH() {
}

func (Handler) DELETE() *http.Response {
	return &http.Response{StatusCode: http.StatusAccepted}
}

func (Handler) OPTIONS() error {
	return YonResourceError{}
}
"#;
}

#[tokio::test]
async fn a_go_worker_may_leave_out_the_request_and_return_every_answer_shape() {
    if !go_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let handler = source(&root, "go", GoShapes::SOURCE);
    let supervisor = supervisor(HandlerSupervisorOptions {
        default_timeout: Duration::from_secs(10),
        ..HandlerSupervisorOptions::default()
    });
    let cancellation = HandlerCancellation::default();
    let mut answers = Vec::new();
    for method in [
        HttpMethod::Get,
        HttpMethod::Post,
        HttpMethod::Put,
        HttpMethod::Patch,
        HttpMethod::Delete,
        HttpMethod::Options,
    ] {
        let answer = supervisor
            .invoke(&handler, &request("go_shape", method, None), &cancellation)
            .await
            .expect("route answer");
        answers.push(answer);
    }
    assert_eq!(answers[0].status, 200);
    assert_eq!(body_json(&answers[0])["ok"], true);
    assert_eq!(body_json(&answers[1])["method"], "POST");
    assert_eq!(answers[2].status, 200);
    assert_eq!(body_json(&answers[2]), serde_json::Value::Null);
    assert_eq!(answers[3].status, 200);
    assert_eq!(body_json(&answers[3]), serde_json::Value::Null);
    assert_eq!(answers[4].status, 202);
    assert_eq!(answers[5].status, 500);
    assert_eq!(
        body_json(&answers[5])["error"],
        "Yon managed resource operation failed."
    );
}

#[tokio::test]
async fn a_go_compile_error_names_the_line_that_failed() {
    if !go_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let relative = "server/workers/contract.go";
    let path = root.path().join(relative);
    fs::create_dir_all(path.parent().expect("handler parent")).expect("handler directory");
    fs::write(
        &path,
        "type Handler struct{}\n\nfunc (Handler) GET() any {\n\treturn missing()\n}\n",
    )
    .expect("handler source");
    let failure = HandlerSource::discover(root.path(), relative).expect_err("compile error");
    let rendered = failure.to_string();
    assert!(rendered.contains("Handler failed to compile"), "{rendered}");
    assert!(rendered.contains("handler.go:4"), "{rendered}");
    assert!(rendered.contains("undefined: missing"), "{rendered}");
}

fn write_project_file(root: &tempfile::TempDir, relative: &str, contents: &str) {
    let path = root.path().join(relative);
    fs::create_dir_all(path.parent().expect("file parent")).expect("file directory");
    fs::write(path, contents).expect("project file");
}

struct GoSharedCode;

impl GoSharedCode {
    const ORDERS: &str = "type OrderService struct{}\n\nfunc (OrderService) Place(item string) string {\n\treturn \"placed \" + item\n}\n";
    const TAX: &str = "package main\ntype Tax struct{}\nfunc (Tax) WithTax(price int) int {\n\treturn price + price/10\n}\n";
    const ROUTE: &str = "type Handler struct{}\n\nfunc (Handler) GET() any {\n\treturn map[string]any{\"order\": OrderService{}.Place(\"anvil\"), \"price\": Tax{}.WithTax(100)}\n}\n";
}

#[tokio::test]
async fn a_go_worker_calls_plain_code_from_server_shared() {
    if !go_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    write_project_file(&root, "server/shared/orders.go", GoSharedCode::ORDERS);
    write_project_file(&root, "server/shared/pricing/tax.go", GoSharedCode::TAX);
    let handler = source(&root, "go", GoSharedCode::ROUTE);
    let supervisor = supervisor(HandlerSupervisorOptions {
        default_timeout: Duration::from_secs(10),
        ..HandlerSupervisorOptions::default()
    });
    let cancellation = HandlerCancellation::default();
    let get = request("go_shared", HttpMethod::Get, None);
    let answered = supervisor
        .invoke(&handler, &get, &cancellation)
        .await
        .expect("GET");
    assert_eq!(body_json(&answered)["order"], "placed anvil");
    assert_eq!(body_json(&answered)["price"], 110);

    let changed = GoSharedCode::ORDERS.replace("placed ", "sent ");
    write_project_file(&root, "server/shared/orders.go", &changed);
    let rebuilt =
        HandlerSource::discover(root.path(), "server/workers/contract.go").expect("rebuilt source");
    let answered = supervisor
        .invoke(&rebuilt, &get, &cancellation)
        .await
        .expect("GET after the shared file changed");
    assert_eq!(body_json(&answered)["order"], "sent anvil");
}

#[test]
fn a_go_shared_file_error_names_the_shared_file() {
    if !go_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let relative = "server/workers/contract.go";
    write_project_file(&root, relative, GoSharedCode::ROUTE);
    write_project_file(
        &root,
        "server/shared/pricing/tax.go",
        "package main\ntype Tax struct{}\nfunc (Tax) WithTax(price int) int {\n\treturn missing(price)\n}\n",
    );
    write_project_file(&root, "server/shared/orders.go", GoSharedCode::ORDERS);
    let failure = HandlerSource::discover(root.path(), relative).expect_err("compile error");
    let rendered = failure.to_string();
    assert!(
        rendered.contains("server/shared/pricing/tax.go:4"),
        "{rendered}"
    );

    write_project_file(
        &root,
        "server/shared/pricing/tax.go",
        "package pricing\ntype Tax struct{}\nfunc (Tax) WithTax(price int) int {\n\treturn price\n}\n",
    );
    let failure = HandlerSource::discover(root.path(), relative).expect_err("other package");
    let rendered = failure.to_string();
    assert!(rendered.contains("TY1310"), "{rendered}");
    assert!(rendered.contains("package main"), "{rendered}");

    write_project_file(
        &root,
        "server/shared/pricing/tax.go",
        "// @Service\ntype TaxService struct{}\n",
    );
    let failure = HandlerSource::discover(root.path(), relative).expect_err("annotated code");
    let rendered = failure.to_string();
    assert!(rendered.contains("TY1310"), "{rendered}");
    assert!(
        rendered.contains("server/shared/pricing/tax.go"),
        "{rendered}"
    );
}

struct GoNativePackages;

impl GoNativePackages {
    const MODULE: &str = "module example.com/shop/packages\n\ngo 1.27\n";
    const PRICING: &str = "package pricing\n\nimport \"example.com/shop/packages/rates\"\n\nfunc WithTax(price int) int {\n\treturn price + price*rates.Percent/100\n}\n";
    const RATES: &str =
        "package rates\n\ntype Table struct{ Region string }\n\nconst Percent = 10\n";
    const ROUTE: &str = "import \"example.com/shop/packages/pricing\"\n\ntype Handler struct{}\n\nfunc (Handler) GET() any {\n\treturn map[string]any{\"price\": pricing.WithTax(100)}\n}\n";
}

#[tokio::test]
async fn a_go_worker_imports_a_native_package_from_server_packages() {
    if !go_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    write_project_file(&root, "server/packages/go.mod", GoNativePackages::MODULE);
    write_project_file(
        &root,
        "server/packages/pricing/tax.go",
        GoNativePackages::PRICING,
    );
    write_project_file(
        &root,
        "server/packages/rates/rates.go",
        GoNativePackages::RATES,
    );
    let handler = source(&root, "go", GoNativePackages::ROUTE);
    let supervisor = supervisor(HandlerSupervisorOptions {
        default_timeout: Duration::from_secs(10),
        ..HandlerSupervisorOptions::default()
    });
    let cancellation = HandlerCancellation::default();
    let get = request("go_native", HttpMethod::Get, None);
    let answered = supervisor
        .invoke(&handler, &get, &cancellation)
        .await
        .expect("GET");
    assert_eq!(body_json(&answered)["price"], 110);

    write_project_file(
        &root,
        "server/packages/rates/rates.go",
        "package rates\n\ntype Table struct{ Region string }\n\nconst Percent = 20\n",
    );
    let rebuilt =
        HandlerSource::discover(root.path(), "server/workers/contract.go").expect("rebuilt source");
    let answered = supervisor
        .invoke(&rebuilt, &get, &cancellation)
        .await
        .expect("GET after the native package changed");
    assert_eq!(body_json(&answered)["price"], 120);
}

struct GoVendoredModule;

impl GoVendoredModule {
    const MODULE: &str =
        "module example.com/shop/packages\n\ngo 1.27\n\nrequire example.com/greet v1.0.0\n";
    const SUMS: &str = "example.com/greet v1.0.0 h1:Zm9yLXRlc3RzLW9ubHk=\nexample.com/greet v1.0.0/go.mod h1:Zm9yLXRlc3RzLW9ubHk=\n";
    const LIST: &str = "# example.com/greet v1.0.0\n## explicit; go 1.22\nexample.com/greet\n";
    const GREET: &str =
        "package greet\n\nfunc Hello(name string) string {\n\treturn \"hello \" + name\n}\n";
    const LABELS: &str = "package labels\n\nimport \"example.com/greet\"\n\nfunc Shelf(name string) string {\n\treturn greet.Hello(name) + \" shelf\"\n}\n";
    const ROUTE: &str = "import (\n\t\"example.com/greet\"\n\t\"example.com/shop/packages/labels\"\n)\n\ntype Handler struct{}\n\nfunc (Handler) GET() any {\n\treturn map[string]any{\"greeting\": greet.Hello(\"anvil\"), \"label\": labels.Shelf(\"anvil\")}\n}\n";

    fn write(root: &tempfile::TempDir) {
        for (relative, contents) in [
            ("server/packages/go.mod", Self::MODULE),
            ("server/packages/go.sum", Self::SUMS),
            ("server/packages/vendor/modules.txt", Self::LIST),
            (
                "server/packages/vendor/example.com/greet/greet.go",
                Self::GREET,
            ),
            ("server/packages/labels/labels.go", Self::LABELS),
        ] {
            write_project_file(root, relative, contents);
        }
    }
}

#[tokio::test]
async fn a_go_worker_imports_a_vendored_third_party_module_offline() {
    if !go_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    GoVendoredModule::write(&root);
    let handler = source(&root, "go", GoVendoredModule::ROUTE);
    let supervisor = supervisor(HandlerSupervisorOptions {
        default_timeout: Duration::from_secs(10),
        ..HandlerSupervisorOptions::default()
    });
    let cancellation = HandlerCancellation::default();
    let get = request("go_vendored", HttpMethod::Get, None);
    let answered = supervisor
        .invoke(&handler, &get, &cancellation)
        .await
        .expect("GET");
    assert_eq!(body_json(&answered)["greeting"], "hello anvil");
    assert_eq!(body_json(&answered)["label"], "hello anvil shelf");

    write_project_file(
        &root,
        "server/packages/vendor/example.com/greet/greet.go",
        "package greet\n\nfunc Hello(name string) string {\n\treturn \"hi \" + name\n}\n",
    );
    let rebuilt =
        HandlerSource::discover(root.path(), "server/workers/contract.go").expect("rebuilt source");
    let answered = supervisor
        .invoke(&rebuilt, &get, &cancellation)
        .await
        .expect("GET after the vendored module changed");
    assert_eq!(body_json(&answered)["greeting"], "hi anvil");
}

#[test]
fn a_go_worker_with_a_stale_vendor_folder_fails_with_ty1368() {
    if !go_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    GoVendoredModule::write(&root);
    write_project_file(
        &root,
        "server/packages/go.mod",
        "module example.com/shop/packages\n\ngo 1.27\n\nrequire example.com/greet v1.1.0\n",
    );
    let relative = "server/workers/contract.go";
    write_project_file(&root, relative, GoVendoredModule::ROUTE);

    let failure = HandlerSource::discover(root.path(), relative).expect_err("stale vendor");

    let rendered = failure.to_string();
    assert!(rendered.contains("TY1368"), "{rendered}");
    assert!(
        rendered.contains("inconsistent vendoring in server/packages"),
        "{rendered}"
    );
    assert!(rendered.contains("go mod vendor"), "{rendered}");
}

#[test]
fn a_go_worker_that_imports_an_unvendored_module_fails_with_ty1368() {
    if !go_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let relative = "server/workers/contract.go";
    write_project_file(
        &root,
        relative,
        "import \"gopkg.in/yaml.v3\"\n\ntype Handler struct{}\n\nfunc (Handler) GET() any {\n\treturn yaml.Marshal\n}\n",
    );

    let failure = HandlerSource::discover(root.path(), relative).expect_err("not vendored");

    let rendered = failure.to_string();
    assert!(rendered.contains("TY1368"), "{rendered}");
    assert!(rendered.contains("gopkg.in/yaml.v3"), "{rendered}");
}

struct NativePeer;

impl NativePeer {
    const HANDLERS: [(&str, &str, &str); 4] = [
        (
            "php",
            "php-cgi",
            "<?php\nfinal class Handler\n{\n    public static function GET(): void\n    {\n        echo $_SERVER['REMOTE_ADDR'];\n    }\n}\n",
        ),
        (
            "go",
            "go",
            "import \"net/http\"\n\ntype Handler struct{}\n\nfunc (Handler) GET(request *http.Request) any {\n\treturn request.RemoteAddr\n}\n",
        ),
        (
            "java",
            "javac",
            "\nclass Handler {\n    static void GET(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {\n        byte[] body = exchange.getRemoteAddress().getAddress().getHostAddress().getBytes();\n        exchange.sendResponseHeaders(200, body.length);\n        exchange.getResponseBody().write(body);\n    }\n}\n",
        ),
        (
            "kt",
            "kotlinc",
            "\nobject Handler {\n    @JvmStatic fun GET(exchange: com.sun.net.httpserver.HttpExchange) {\n        val body = exchange.remoteAddress.address.hostAddress.toByteArray()\n        exchange.sendResponseHeaders(200, body.size.toLong())\n        exchange.responseBody.write(body)\n    }\n}\n",
        ),
    ];
    const CLIENT: &str = "203.0.113.9";
}

#[tokio::test]
async fn native_requests_report_the_client_address_that_ty_resolved() {
    let supervisor = supervisor(HandlerSupervisorOptions {
        default_timeout: Duration::from_mins(2),
        ..HandlerSupervisorOptions::default()
    });
    for (extension, tool, contents) in NativePeer::HANDLERS {
        if !available(tool) {
            eprintln!("skipping {extension}: {tool} is unavailable");
            continue;
        }
        let root = tempfile::tempdir().expect("project");
        let handler = source(&root, extension, contents);
        let mut get = request("native_peer", HttpMethod::Get, None);
        get.headers.insert(
            String::from("x-real-ip"),
            vec![String::from(NativePeer::CLIENT)],
        );
        let answered = supervisor
            .invoke(&handler, &get, &HandlerCancellation::default())
            .await
            .expect("GET");
        let body = answered.body.expect("peer body").data;
        assert!(body.contains(NativePeer::CLIENT), "{extension}: {body}");
    }
}

#[tokio::test]
async fn environment_is_denied_by_default_and_explicitly_allowlisted() {
    let candidates = ["HOME", "USERPROFILE", "CARGO_HOME", "CI"];
    let (name, expected) = candidates
        .iter()
        .find_map(|name| std::env::var(name).ok().map(|value| (*name, value)))
        .expect("test host needs one non-baseline environment variable");
    let root = tempfile::tempdir().expect("project");
    let source = source(
        &root,
        "js",
        r"
export class Handler {
  static async POST(request) {
    const name = await request.text()
    return { present: Object.hasOwn(process.env, name), value: process.env[name] ?? null }
  }
}
",
    );
    let invocation = request("environment", HttpMethod::Post, Some(name));
    let denied = supervisor(HandlerSupervisorOptions::default())
        .invoke(&source, &invocation, &HandlerCancellation::default())
        .await
        .expect("denied environment response");
    assert_eq!(body_json(&denied)["present"], false);

    let environment = EnvironmentPolicy::from_names([name]).expect("allowlist");
    let allowed = supervisor(HandlerSupervisorOptions {
        environment,
        ..HandlerSupervisorOptions::default()
    })
    .invoke(&source, &invocation, &HandlerCancellation::default())
    .await
    .expect("allowed environment response");
    let allowed = body_json(&allowed);
    assert_eq!(allowed["present"], true);
    assert_eq!(allowed["value"], expected);
}

#[tokio::test]
async fn adapter_authoring_failures_are_bounded_protocol_responses() {
    let root = tempfile::tempdir().expect("project");
    let javascript = source(
        &root,
        "js",
        "export class Handler { static GET() { return { ok: true } } }",
    );
    let missing = supervisor(HandlerSupervisorOptions::default())
        .invoke(
            &javascript,
            &request("missing", HttpMethod::Delete, None),
            &HandlerCancellation::default(),
        )
        .await
        .expect("protocol error response");
    assert_eq!(missing.status, 405);
    assert_eq!(missing.error.expect("error").code, "TY2202");

    let python = source(
        &root,
        "py",
        r#"
class Handler:
    @staticmethod
    def GET(request):
        raise RuntimeError("bounded failure")
"#,
    );
    let failed = supervisor(HandlerSupervisorOptions::default())
        .invoke(
            &python,
            &request("failed", HttpMethod::Get, None),
            &HandlerCancellation::default(),
        )
        .await
        .expect("protocol error response");
    let error = failed.error.expect("error");
    assert_eq!(error.code, "TY2201");
    assert_eq!(error.message, "bounded failure");
    assert!(!error.message.contains("Traceback"));

    let instance = source(
        &root,
        "py",
        r"
class Handler:
    def GET(self):
        return {'invalid': True}
",
    );
    let rejected = supervisor(HandlerSupervisorOptions::default())
        .invoke(
            &instance,
            &request("instance", HttpMethod::Get, None),
            &HandlerCancellation::default(),
        )
        .await
        .expect("authoring response");
    assert_eq!(rejected.error.expect("error").code, "TY2202");

    let invalid_root = tempfile::tempdir().expect("invalid source project");
    let missing_class_path = invalid_root.path().join("server/workers/missing.js");
    fs::create_dir_all(missing_class_path.parent().expect("handler parent"))
        .expect("handler directory");
    fs::write(&missing_class_path, "export const notAHandler = true")
        .expect("invalid handler source");
    let rejected = HandlerSource::discover(invalid_root.path(), "server/workers/missing.js")
        .expect_err("missing Handler is rejected before runtime");
    assert!(rejected.to_string().contains("TY1310"), "{rejected}");

    let invalid_python = source(
        &root,
        "py",
        "class Handler:\n    @staticmethod\n    def GET(request):\n        return +\n",
    );
    let rejected = supervisor(HandlerSupervisorOptions::default())
        .invoke(
            &invalid_python,
            &request("invalid_python", HttpMethod::Get, None),
            &HandlerCancellation::default(),
        )
        .await
        .expect("adapter syntax response");
    assert_eq!(rejected.error.expect("error").code, "TY2201");

    let non_serializable = source(
        &root,
        "js",
        "export class Handler { static GET() { return 1n } }",
    );
    let rejected = supervisor(HandlerSupervisorOptions::default())
        .invoke(
            &non_serializable,
            &request("non_serializable", HttpMethod::Get, None),
            &HandlerCancellation::default(),
        )
        .await
        .expect("serialization response");
    assert_eq!(rejected.error.expect("error").code, "TY2203");
}

#[test]
fn a_worker_without_a_class_reports_the_missing_class() {
    let root = tempfile::tempdir().expect("worker project");
    let path = root.path().join("server/workers/empty.js");
    fs::create_dir_all(path.parent().expect("worker parent")).expect("worker directory");
    fs::write(&path, "// no class here\n").expect("worker source");
    let rejected = HandlerSource::discover(root.path(), "server/workers/empty.js")
        .expect_err("a worker without a class has no Handler");
    let diagnostic = &rejected.diagnostics()[0];
    assert_eq!(diagnostic.code.number(), 1310, "{rejected}");
    assert!(
        diagnostic.message.contains("found no class declaration"),
        "{rejected}"
    );
}

#[tokio::test]
async fn timeout_cancellation_crash_and_recovery_reap_each_process() {
    let root = tempfile::tempdir().expect("project");
    let source = source(
        &root,
        "js",
        r"
export class Handler {
  static async POST(request) {
    const mode = (await request.text()) || undefined
    if (mode === 'crash') process.exit(23)
    if (mode === 'block') while (true) {}
    if (mode === 'sleep') await new Promise((resolve) => setTimeout(resolve, 5000))
    return { mode: mode ?? 'ok' }
  }
}
",
    );
    let bounded = supervisor(HandlerSupervisorOptions {
        default_timeout: Duration::from_millis(75),
        ..HandlerSupervisorOptions::default()
    });
    let already_cancelled = HandlerCancellation::default();
    already_cancelled.cancel();
    let early = bounded
        .invoke(
            &source,
            &request("early_cancel", HttpMethod::Post, None),
            &already_cancelled,
        )
        .await
        .expect_err("early cancellation");
    assert!(early.to_string().contains("TY2111"));

    let timed_out = bounded
        .invoke(
            &source,
            &request("timeout", HttpMethod::Post, Some("sleep")),
            &HandlerCancellation::default(),
        )
        .await
        .expect_err("timeout");
    assert!(timed_out.to_string().contains("TY2110"));

    let forced = supervisor(HandlerSupervisorOptions {
        default_timeout: Duration::from_millis(40),
        cancellation_grace: Duration::from_millis(20),
        ..HandlerSupervisorOptions::default()
    })
    .invoke(
        &source,
        &request("forced", HttpMethod::Post, Some("block")),
        &HandlerCancellation::default(),
    )
    .await
    .expect_err("forced termination");
    assert!(forced.to_string().contains("TY2110"));

    let cancellation = HandlerCancellation::default();
    let trigger = cancellation.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(40)).await;
        trigger.cancel();
    });
    let cancelled = supervisor(HandlerSupervisorOptions::default())
        .invoke(
            &source,
            &request("cancel", HttpMethod::Post, Some("sleep")),
            &cancellation,
        )
        .await
        .expect_err("cancellation");
    assert!(cancelled.to_string().contains("TY2111"));
    assert!(cancellation.is_cancelled());

    let healthy = supervisor(HandlerSupervisorOptions::default());
    let crashed = healthy
        .invoke(
            &source,
            &request("crash", HttpMethod::Post, Some("crash")),
            &HandlerCancellation::default(),
        )
        .await
        .expect_err("crash");
    assert!(crashed.to_string().contains("TY2104"));
    let recovered = healthy
        .invoke(
            &source,
            &request("recovered", HttpMethod::Post, None),
            &HandlerCancellation::default(),
        )
        .await
        .expect("fresh process after crash");
    assert_eq!(body_json(&recovered)["mode"], "ok");
}

#[tokio::test]
async fn inheriting_descendants_are_bounded_and_reaped_for_every_outcome() {
    let root = tempfile::tempdir().expect("project");
    let source = source(
        &root,
        "js",
        r"
import { spawn } from 'node:child_process'
import { writeFileSync } from 'node:fs'

export class Handler {
  static async POST(request) {
    const descendant = spawn(
      process.execPath,
      ['-e', 'setInterval(() => {}, 1000)'],
      { stdio: 'inherit' },
    )
    writeFileSync(request.headers.get('x-pid-file'), String(descendant.pid))
    if ((await request.text()) !== 'success') await new Promise(() => {})
    return { descendant: descendant.pid }
  }
}

",
    );

    let success_pid_file = root.path().join("success.pid");
    let success_supervisor = supervisor(HandlerSupervisorOptions {
        default_timeout: DescendantDeadline::STARTUP,
        cancellation_grace: Duration::from_millis(25),
        ..HandlerSupervisorOptions::default()
    });
    let started = tokio::time::Instant::now();
    let success = tokio::time::timeout(
        DescendantDeadline::STARTUP + DescendantDeadline::SETTLE,
        success_supervisor.invoke(
            &source,
            &descendant_request("descendant_success", "success", &success_pid_file),
            &HandlerCancellation::default(),
        ),
    )
    .await
    .expect("successful invocation stayed bounded")
    .expect("successful handler response");
    assert!(started.elapsed() < DescendantDeadline::STARTUP);
    assert_eq!(success.status, 200, "{:?}", success.error);
    let success_pid = descendant_pid(&success_pid_file);
    assert_eq!(body_json(&success)["descendant"], success_pid);
    assert_process_gone(success_pid).await;

    let timeout_pid_file = root.path().join("timeout.pid");
    let timeout_supervisor = supervisor(HandlerSupervisorOptions {
        default_timeout: DescendantDeadline::STARTUP,
        cancellation_grace: Duration::from_millis(25),
        ..HandlerSupervisorOptions::default()
    });
    let timed_out = tokio::time::timeout(
        DescendantDeadline::STARTUP + DescendantDeadline::SETTLE,
        timeout_supervisor.invoke(
            &source,
            &descendant_request("descendant_timeout", "hang", &timeout_pid_file),
            &HandlerCancellation::default(),
        ),
    )
    .await
    .expect("timed-out invocation stayed bounded")
    .expect_err("handler deadline");
    assert!(timed_out.to_string().contains("TY2110"));
    let timeout_pid = descendant_pid(&timeout_pid_file);
    assert_process_gone(timeout_pid).await;

    let cancellation_pid_file = root.path().join("cancel.pid");
    let cancellation = HandlerCancellation::default();
    let trigger = cancellation.clone();
    let trigger_pid_file = cancellation_pid_file.clone();
    tokio::spawn(async move {
        let give_up = tokio::time::Instant::now() + DescendantDeadline::STARTUP;
        while !trigger_pid_file.is_file() && tokio::time::Instant::now() < give_up {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        trigger.cancel();
    });
    let cancelled = tokio::time::timeout(
        DescendantDeadline::STARTUP + DescendantDeadline::SETTLE,
        supervisor(HandlerSupervisorOptions {
            default_timeout: DescendantDeadline::STARTUP * 2,
            cancellation_grace: Duration::from_millis(25),
            ..HandlerSupervisorOptions::default()
        })
        .invoke(
            &source,
            &descendant_request("descendant_cancel", "hang", &cancellation_pid_file),
            &cancellation,
        ),
    )
    .await
    .expect("cancelled invocation stayed bounded")
    .expect_err("handler cancellation");
    assert!(cancelled.to_string().contains("TY2111"));
    let cancellation_pid = descendant_pid(&cancellation_pid_file);
    assert_process_gone(cancellation_pid).await;
}

fn streaming_descendant_source(root: &tempfile::TempDir) -> HandlerSource {
    source(
        root,
        "py",
        r#"
import subprocess
import sys
import time

class Handler:
    @staticmethod
    def POST(request):
        descendant = subprocess.Popen(
            [sys.executable, "-c", "import time; time.sleep(60)"],
            stdin=sys.stdin,
            stdout=sys.stdout,
            stderr=sys.stderr,
        )
        with open(request.get_header("X-pid-file"), "w") as target:
            target.write(str(descendant.pid))
        yield {"descendant": descendant.pid}
        while True:
            time.sleep(1)
"#,
    )
}

#[tokio::test]
async fn streaming_deadlines_reap_inheriting_descendants() {
    let root = tempfile::tempdir().expect("project");
    let source = streaming_descendant_source(&root);
    let pid_file = root.path().join("stream-timeout.pid");
    let supervisor = supervisor(HandlerSupervisorOptions {
        default_timeout: DescendantDeadline::STARTUP,
        cancellation_grace: Duration::from_millis(25),
        ..HandlerSupervisorOptions::default()
    });
    let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
    let request = descendant_request("stream_timeout", "hang", &pid_file);
    let invocation = supervisor.invoke_streaming(&source, &request, sender);
    let (result, first) = tokio::join!(invocation, receiver.recv());
    assert!(first.is_some(), "stream should yield before its deadline");
    let failure = result.expect_err("stream deadline");
    assert!(failure.to_string().contains("TY2110"), "{failure}");
    assert_process_gone(descendant_pid(&pid_file)).await;
}

#[tokio::test]
async fn closing_a_stream_subscription_reaps_the_process_group() {
    let root = tempfile::tempdir().expect("project");
    let source = streaming_descendant_source(&root);
    let pid_file = root.path().join("stream-disconnect.pid");
    let supervisor = supervisor(HandlerSupervisorOptions {
        default_timeout: Duration::from_secs(3),
        ..HandlerSupervisorOptions::default()
    });
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    let request = descendant_request("stream_disconnect", "hang", &pid_file);
    let task =
        tokio::spawn(async move { supervisor.invoke_streaming(&source, &request, sender).await });
    receiver.recv().await.expect("first streamed event");
    drop(receiver);
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .expect("disconnect cleanup stayed bounded")
        .expect("stream task joined")
        .expect("subscriber close is a clean stop");
    assert_process_gone(descendant_pid(&pid_file)).await;
}

#[tokio::test]
async fn oversized_python_stream_event_emits_only_a_bounded_failure_frame() {
    let root = tempfile::tempdir().expect("project");
    let source = source(
        &root,
        "py",
        r#"
class Handler:
    @staticmethod
    def GET(_request):
        yield "x" * (300 * 1024)
"#,
    );
    let supervisor = supervisor(HandlerSupervisorOptions::default());
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    let result = supervisor
        .invoke_streaming(
            &source,
            &request("oversized_python_stream", HttpMethod::Get, None),
            sender,
        )
        .await
        .expect_err("oversized stream event");
    assert!(result.to_string().contains("TY2107"), "{result}");
    assert!(
        receiver.recv().await.is_none(),
        "oversized event must not escape"
    );
}

#[tokio::test]
async fn a_slow_subscriber_applies_backpressure_until_the_stream_deadline() {
    let root = tempfile::tempdir().expect("project");
    let source = source(
        &root,
        "py",
        r#"
class Handler:
    @staticmethod
    def GET(_request):
        while True:
            yield "x" * (200 * 1024)
"#,
    );
    BackpressureDeadline::assert_bounded_with_a_full_queue(
        &source,
        "slow_subscriber",
        "a full subscriber queue must remain deadline bounded",
    )
    .await;
}

#[tokio::test]
async fn a_fast_javascript_generator_obeys_stdout_and_subscriber_backpressure() {
    let runtime = HandlerSupervisorOptions::default().runtimes.javascript;
    if !available(&runtime.to_string_lossy()) {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(
        &root,
        "js",
        r#"
export class Handler {
  static async *GET(_request) {
    while (true) yield "x".repeat(200 * 1024)
  }
}
"#,
    );
    BackpressureDeadline::assert_bounded_with_a_full_queue(
        &source,
        "javascript_slow_subscriber",
        "a fast generator must remain deadline bounded",
    )
    .await;
}

#[tokio::test]
async fn compiled_relay_bounds_sideband_redacts_errors_and_reaps_descendants() {
    if !available("php-cgi") || !available("python3") {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    write_compiled_relay_delegate(&root);
    let relay_sources = RelayFixtures::COMPILED;
    let supervisor = supervisor(HandlerSupervisorOptions {
        default_timeout: Duration::from_secs(10),
        ..HandlerSupervisorOptions::default()
    });
    for &(extension, contents) in &relay_sources {
        let tool = match extension {
            "java" => "java",
            "kt" => "kotlinc",
            "cs" => "dotnet",
            "rs" => "rustc",
            "go" => "go",
            _ => "php-cgi",
        };
        if !available(tool) {
            continue;
        }
        let relay = source(&root, extension, contents);
        let response = supervisor
            .invoke(
                &relay,
                &request(
                    &format!("relay_sideband_{extension}"),
                    HttpMethod::Post,
                    Some("sideband"),
                ),
                &HandlerCancellation::default(),
            )
            .await
            .expect("relay produces a sanitized upstream response");
        assert_eq!(response.status, 502, "{extension}");
        let rendered = response.body.expect("error body").data;
        assert!(
            !rendered.contains("secret-canary"),
            "{extension}: {rendered}"
        );
        assert!(!rendered.contains("xxxx"), "{extension}: {rendered}");
    }

    let source = source(&root, "php", relay_sources[0].1);
    let pid_file = root.path().join("relay-descendant.pid");
    let mut invocation = descendant_request("relay_timeout", "hang", &pid_file);
    invocation.method = HttpMethod::Post;
    invocation.deadline_ms = Some(250);
    let response = supervisor
        .invoke(&source, &invocation, &HandlerCancellation::default())
        .await
        .expect("the relay answers its own deadline");
    assert_eq!(response.status, 502);
    assert_process_gone(descendant_pid(&pid_file)).await;
}

#[tokio::test]
async fn interpreted_relays_bound_both_streams_and_redact_process_details() {
    if !available("python3") {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let helper = root.path().join("server/helpers/probe.py");
    fs::create_dir_all(helper.parent().expect("helper parent")).expect("helpers");
    fs::write(
        &helper,
        r#"import json
import sys
request = json.load(sys.stdin)
mode = request.get("body", {}).get("data", "")
if mode == "stdout":
    sys.stdout.write("secret-canary:" + "x" * (17 * 1024 * 1024))
    sys.stdout.flush()
else:
    sys.stderr.write("secret-canary:" + "x" * (128 * 1024))
    sys.stderr.flush()
    sys.exit(9)
"#,
    )
    .expect("helper");
    let cases = [
        (
            "py",
            r#"
class Handler:
    @staticmethod
    def POST(request):
        return relay(("python3", "server/helpers/probe.py"), request)
"#,
            true,
        ),
        (
            "js",
            r"
export class Handler {
  static POST(request) { return relay(['python3', 'server/helpers/probe.py'], request) }
}
",
            available("node"),
        ),
    ];
    let supervisor = supervisor(HandlerSupervisorOptions::default());
    for (extension, contents, supported) in cases {
        if !supported {
            continue;
        }
        let relay = source(&root, extension, contents);
        for mode in ["sideband", "stdout"] {
            let response = supervisor
                .invoke(
                    &relay,
                    &request(
                        &format!("interpreted_relay_{extension}_{mode}"),
                        HttpMethod::Post,
                        Some(mode),
                    ),
                    &HandlerCancellation::default(),
                )
                .await
                .expect("relay returns a sanitized upstream response");
            assert_eq!(response.status, 502, "{extension}:{mode}");
            let rendered = response.body.expect("error body").data;
            assert!(
                !rendered.contains("secret-canary"),
                "{extension}: {rendered}"
            );
            assert!(!rendered.contains("python3"), "{extension}: {rendered}");
            assert!(rendered.len() < 1024, "{extension}: {rendered}");
        }
    }
}

#[tokio::test]
async fn concurrency_limit_includes_queue_time_in_each_request_deadline() {
    let root = tempfile::tempdir().expect("project");
    let source = source(
        &root,
        "js",
        r"
export class Handler {
  static async GET() {
    await new Promise((resolve) => setTimeout(resolve, 150))
    return { completed: true }
  }
}
",
    );
    let supervisor = supervisor(HandlerSupervisorOptions {
        default_timeout: Duration::from_secs(5),
        max_concurrency: 1,
        ..HandlerSupervisorOptions::default()
    });

    let first_supervisor = supervisor.clone();
    let first_source = source.clone();
    let first = tokio::spawn(async move {
        first_supervisor
            .invoke(
                &first_source,
                &request("first", HttpMethod::Get, None),
                &HandlerCancellation::default(),
            )
            .await
    });
    tokio::time::sleep(Duration::from_millis(25)).await;

    let mut queued_request = request("queued", HttpMethod::Get, None);
    queued_request.deadline_ms = Some(40);
    let queued = supervisor
        .invoke(&source, &queued_request, &HandlerCancellation::default())
        .await
        .expect_err("queued request deadline");
    assert!(queued.to_string().contains("TY2110"));

    let completed = first.await.expect("first task").expect("first response");
    assert_eq!(body_json(&completed)["completed"], true);
}

#[tokio::test]
async fn protocol_smuggling_oversize_mismatch_and_stderr_flood_fail_closed() {
    let root = tempfile::tempdir().expect("project");
    let source = source(
        &root,
        "js",
        r"
export class Handler {
  static async POST(request) {
    const mode = await request.text()
    if (mode === 'trailing') {
      process.stdout.write('smuggled')
      return { ok: true }
    }
    if (mode === 'oversized') {
      await new Promise((resolve) => {
        process.stdout.write(Buffer.alloc(16 * 1024 * 1024 + 5), () => process.exit(0))
      })
    }
    if (mode === 'mismatch') {
      const payload = Buffer.from(JSON.stringify({
        protocol_version: 1,
        kind: 'response',
        request_id: 'another_request',
        status: 200,
        headers: {},
        body: { encoding: 'utf8', data: 'null' },
      }))
      const prefix = Buffer.alloc(4)
      prefix.writeUInt32BE(payload.length)
      await new Promise((resolve) => {
        process.stdout.write(Buffer.concat([prefix, payload]), () => process.exit(0))
      })
    }
    if (mode === 'stderr') {
      await new Promise((resolve) => process.stderr.write('x'.repeat(70 * 1024), resolve))
      return { ok: true }
    }
    return { ok: true }
  }
}
",
    );
    let supervisor = supervisor(HandlerSupervisorOptions::default());
    for (mode, code) in [
        ("trailing", "TY2103"),
        ("oversized", "TY2103"),
        ("mismatch", "TY2102"),
        ("stderr", "TY2107"),
    ] {
        let failure = supervisor
            .invoke(
                &source,
                &request(mode, HttpMethod::Post, Some(mode)),
                &HandlerCancellation::default(),
            )
            .await
            .expect_err("protocol failure");
        assert!(failure.to_string().contains(code), "{mode}: {failure}");
    }
}

fn write_compiled_relay_delegate(root: &tempfile::TempDir) {
    let helper = root.path().join("server/helpers/probe.py");
    fs::create_dir_all(helper.parent().expect("helper parent")).expect("helpers");
    fs::write(
        &helper,
        r#"import json
import subprocess
import sys
import time

request = json.load(sys.stdin)
mode = request.get("body", {}).get("data", "")
if mode == "sideband":
    sys.stderr.write("secret-canary:" + "x" * (128 * 1024))
    sys.stderr.flush()
    sys.exit(9)
else:
    descendant = subprocess.Popen(
        [sys.executable, "-c", "import time; time.sleep(60)"],
        stdin=sys.stdin,
        stdout=sys.stdout,
        stderr=sys.stderr,
    )
    with open(request["headers"]["x-pid-file"][0], "w") as target:
        target.write(str(descendant.pid))
    time.sleep(60)
"#,
    )
    .expect("helper");
}

struct RelayFixtures;

impl RelayFixtures {
    const COMPILED: [(&str, &str); 6] = [
        (
            "php",
            r#"<?php
final class Handler
{
    public static function POST(): void
    {
        Yon::relay(["python3", "server/helpers/probe.py"]);
    }
}
"#,
        ),
        (
            "java",
            r#"
class Handler {
    static void POST(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        Yon.relay(java.util.List.of("python3", "server/helpers/probe.py"), exchange);
    }
}
"#,
        ),
        (
            "kt",
            r#"
object Handler {
    @JvmStatic fun POST(exchange: com.sun.net.httpserver.HttpExchange) =
        Yon.relay(listOf("python3", "server/helpers/probe.py"), exchange)
}
"#,
        ),
        (
            "cs",
            r#"
sealed class Handler {
    public static HttpResponseMessage POST(HttpRequestMessage request) =>
        Yon.Relay(new[] { "python3", "server/helpers/probe.py" }, request);
}
"#,
        ),
        (
            "rs",
            r#"
struct Handler;
impl Handler {
    fn POST(request: &http::Request<Vec<u8>>) -> http::Response<Vec<u8>> {
        relay(&["python3", "server/helpers/probe.py"], request)
    }
}
"#,
        ),
        (
            "go",
            r#"
import "net/http"

type Handler struct{}

func (Handler) POST(request *http.Request) *http.Response {
	return Relay([]string{"python3", "server/helpers/probe.py"}, request)
}
"#,
        ),
    ];
}
