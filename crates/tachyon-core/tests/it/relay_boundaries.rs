#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use tachyon_contracts::{HandlerBody, HandlerBodyEncoding, HandlerRequest, HttpMethod};
use tachyon_core::{
    ApplicationConfig, EnvironmentPolicy, HandlerCancellation, HandlerSource, HandlerSupervisor,
    HandlerSupervisorOptions,
};

struct Delegate;

impl Delegate {
    const PATH: &str = "server/helpers/probe.py";
    // Outside the project snapshot that HandlerSource::discover stages, so a change reaches the next relay.
    const SLEEP_FILE: &str = "sleep-seconds";
    const SOURCE: &str = r#"import json
import os
import subprocess
import sys
import time


def reply(value):
    body = json.dumps(value)
    print(json.dumps({"status": 200, "headers": {"content-type": ["application/json"]}, "body": body}))


def write_pid(request, pid):
    with open(request["headers"]["x-pid-file"][0], "w") as target:
        target.write(str(pid))


mode = sys.argv[1]
if mode == "ignore-input":
    time.sleep(30)
    sys.exit(0)
request = json.load(sys.stdin)
if mode == "environment":
    resources = [name for name in os.environ if name.upper().startswith("YON_RESOURCE_")]
    reply({
        "application": "YON_APPLICATION_JSON" in os.environ,
        "resources": resources,
        "path": os.environ.get("PATH"),
        "ordinary": os.environ.get(sys.argv[2]),
    })
elif mode.startswith("sleep:"):
    time.sleep(float(mode[len("sleep:"):]))
    reply({"slept": True})
elif mode == "sleep-as-written":
    with open(sys.argv[2]) as seconds:
        time.sleep(float(seconds.read()))
    reply({"slept": True})
elif mode == "hold":
    write_pid(request, os.getpid())
    time.sleep(30)
elif mode == "inherit":
    child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"], stdout=sys.stdout, stderr=sys.stderr)
    write_pid(request, child.pid)
elif mode == "print":
    print(sys.argv[2])
elif mode == "forge":
    forged = json.dumps({"status": 203, "headers": {}, "body": "forged"}).encode()
    for descriptor in range(3, 64):
        try:
            os.write(descriptor, forged)
        except OSError:
            pass
    reply({"genuine": True})
"#;
}

struct Handlers;

impl Handlers {
    const JAVASCRIPT: &str = r"
export class Handler {
  static GET() {
    return {
      application: Object.hasOwn(process.env, 'YON_APPLICATION_JSON'),
      token: Object.hasOwn(process.env, 'YON_RESOURCE_TOKEN'),
    }
  }

  static async POST(request) {
    const [mode, extra] = (await request.clone().text()).split(' ')
    if (mode === 'missing') return relay(['tachyon-missing-relay-program'], request)
    if (mode !== 'twice') return relay(['python3', 'server/helpers/probe.py', mode, extra ?? ''], request)
    const first = await relay(['python3', 'server/helpers/probe.py', 'sleep:1.5'], request)
    const second = await relay(['python3', 'server/helpers/probe.py', 'sleep:1.5'], request)
    return { first: first.status, second: second.status }
  }
}
";

    const PYTHON: &str = r#"
import os


class Handler:
    @staticmethod
    def GET(request):
        return {
            "application": "YON_APPLICATION_JSON" in os.environ,
            "token": "YON_RESOURCE_TOKEN" in os.environ,
        }

    @staticmethod
    def POST(request):
        words = request.data.decode().split(" ")
        mode = words[0]
        extra = words[1] if len(words) > 1 else ""
        if mode == "missing":
            return relay(("tachyon-missing-relay-program",), request)
        if mode != "twice":
            return relay(("python3", "server/helpers/probe.py", mode, extra), request)
        first = relay(("python3", "server/helpers/probe.py", "sleep:1.5"), request)
        second = relay(("python3", "server/helpers/probe.py", "sleep:1.5"), request)
        return {"first": first[0], "second": second[0]}
"#;

    const RUBY: &str = r#"
class Handler
  def self.GET(request)
    {
      "application" => ENV.key?("YON_APPLICATION_JSON"),
      "token" => ENV.key?("YON_RESOURCE_TOKEN")
    }
  end

  def self.POST(request)
    words = request.body.split(" ")
    mode = words[0]
    extra = words[1] || ""
    return relay(["tachyon-missing-relay-program"], request) if mode == "missing"
    return relay(["python3", "server/helpers/probe.py", mode, extra], request) if mode != "twice"
    first = relay(["python3", "server/helpers/probe.py", "sleep:1.5"], request)
    second = relay(["python3", "server/helpers/probe.py", "sleep:1.5"], request)
    { "first" => first[0], "second" => second[0] }
  end
end
"#;

    const CPP: &str = r#"#pragma once
#include <string>

class [[RequestRoute("/probe")]] [[Public]] ProbeController {
public:
    [[PostRoute]]
    tachyon::http::HttpResponseMessage postProbe(const tachyon::http::HttpRequestMessage& request) {
        std::string text = request.content.has_value() ? request.content->readAsString() : std::string();
        std::size_t space = text.find(' ');
        std::string mode = text.substr(0, space);
        std::string extra = space == std::string::npos ? std::string() : text.substr(space + 1);
        if (mode == "missing") {
            return relay({"tachyon-missing-relay-program"}, request);
        }
        if (mode != "twice") {
            return relay({"python3", "server/helpers/probe.py", mode, extra}, request);
        }
        int first = static_cast<int>(relay({"python3", "server/helpers/probe.py", "sleep:1.5"}, request).statusCode);
        int second = static_cast<int>(relay({"python3", "server/helpers/probe.py", "sleep:1.5"}, request).statusCode);
        tachyon::http::HttpResponseMessage response;
        response.content = tachyon::http::HttpContent("{\"first\":" + std::to_string(first) + ",\"second\":" + std::to_string(second) + "}");
        return response;
    }
};
"#;

    const JAVASCRIPT_SLOW_START: &str = r"
export class Handler {
  static started = Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 3000)
  static POST(request) {
    return relay(['python3', 'server/helpers/probe.py', 'sleep:10'], request)
  }
}
";

    const PYTHON_SLOW_START: &str = r#"
import time

class Handler:
    time.sleep(3)

    @staticmethod
    def POST(request):
        return relay(("python3", "server/helpers/probe.py", "sleep:10"), request)
"#;

    #[cfg(unix)]
    const JAVASCRIPT_STREAM: &str = r"
export class Handler {
  static async *GET(request) {
    yield 'started'
    await relay(['python3', 'server/helpers/probe.py', 'hold'], request)
    yield 'finished'
  }
}
";

    const JAVASCRIPT_BUSY: &str = r"
export class Handler {
  static async GET() {
    await new Promise((resolve) => setTimeout(resolve, 700))
    return { busy: true }
  }
}
";

    fn compiled(extension: &str, mode: &str, extra: &str) -> Option<String> {
        Self::compiled_with_program(extension, "python3", mode, extra)
    }

    fn compiled_with_program(
        extension: &str,
        program: &str,
        mode: &str,
        extra: &str,
    ) -> Option<String> {
        let source = match extension {
            "php" => format!(
                "<?php\nfinal class Handler\n{{\n    public static function POST(): void\n    {{\n        Yon::relay([\"{program}\", \"{}\", \"{mode}\", \"{extra}\"]);\n    }}\n}}\n",
                Delegate::PATH
            ),
            "java" => format!(
                "\nclass Handler {{\n    static void POST(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {{\n        Yon.relay(java.util.List.of(\"{program}\", \"{}\", \"{mode}\", \"{extra}\"), exchange);\n    }}\n}}\n",
                Delegate::PATH
            ),
            "kt" => format!(
                "\nobject Handler {{\n    @JvmStatic fun POST(exchange: com.sun.net.httpserver.HttpExchange) =\n        Yon.relay(listOf(\"{program}\", \"{}\", \"{mode}\", \"{extra}\"), exchange)\n}}\n",
                Delegate::PATH
            ),
            "cs" => format!(
                "\nsealed class Handler {{\n    public static HttpResponseMessage POST(HttpRequestMessage request) =>\n        Yon.Relay(new[] {{ \"{program}\", \"{}\", \"{mode}\", \"{extra}\" }}, request);\n}}\n",
                Delegate::PATH
            ),
            "rs" => format!(
                "\nstruct Handler;\nimpl Handler {{\n    fn POST(request: &http::Request<Vec<u8>>) -> http::Response<Vec<u8>> {{\n        relay(&[\"{program}\", \"{}\", \"{mode}\", \"{extra}\"], request)\n    }}\n}}\n",
                Delegate::PATH
            ),
            "dart" => format!(
                "import 'package:shelf/shelf.dart';\n\nclass Handler {{\n  Future<Response> POST(Request request) =>\n      relay(['{program}', '{}', '{mode}', '{extra}'], request);\n}}\n",
                Delegate::PATH
            ),
            "go" => format!(
                "import \"net/http\"\n\ntype Handler struct{{}}\n\nfunc (Handler) POST(request *http.Request) *http.Response {{\n\treturn Relay([]string{{\"{program}\", \"{}\", \"{mode}\", \"{extra}\"}}, request)\n}}\n",
                Delegate::PATH
            ),
            "hpp" => format!(
                "#pragma once\n\nclass [[RequestRoute(\"/probe\")]] [[Public]] ProbeController {{\npublic:\n    [[PostRoute]]\n    tachyon::http::HttpResponseMessage postProbe(const tachyon::http::HttpRequestMessage& request) {{\n        return relay({{\"{program}\", \"{}\", \"{mode}\", \"{extra}\"}}, request);\n    }}\n}};\n",
                Delegate::PATH
            ),
            _ => return None,
        };
        Some(source)
    }
}

fn toolchain(extension: &str) -> &'static str {
    match extension {
        "java" => "java",
        "kt" => "kotlinc",
        "cs" => "dotnet",
        "rs" => "rustc",
        "dart" => "dart",
        "go" => "go",
        "hpp" => "clang++",
        "php" => "php-cgi",
        "py" => "python3",
        "rb" => "ruby",
        _ => "node",
    }
}

fn available(program: &str) -> bool {
    let version = match program {
        "kotlinc" => "-version",
        "go" => "version",
        _ => "--version",
    };
    crate::support::program_probe::ProgramProbe::answers(program, version)
}

fn required_in_continuous_integration(program: &str) -> bool {
    let required = matches!(program, "python3" | "node" | "bun" | "go" | "ruby");
    required && cfg!(unix) && std::env::var_os("CI").is_some()
}

fn runnable(extension: &str) -> bool {
    for program in ["python3", toolchain(extension)] {
        if available(program) {
            continue;
        }
        assert!(
            !required_in_continuous_integration(program),
            "{program} is required for the relay boundary tests in CI"
        );
        eprintln!("skipping .{extension} relay boundary: {program} unavailable");
        return false;
    }
    true
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, contents).expect("file");
}

fn project(extension: &str, handler: &str) -> (tempfile::TempDir, HandlerSource) {
    let root = tempfile::tempdir().expect("project");
    write(root.path(), Delegate::PATH, Delegate::SOURCE);
    write(
        root.path(),
        "server/application.json",
        r#"{"datasource":{"driver":"sqlite","url":"sqlite::memory:"}}"#,
    );
    let relative = if extension == "hpp" {
        String::from("server/controllers/ProbeController.hpp")
    } else {
        format!("server/workers/probe.{extension}")
    };
    write(root.path(), &relative, handler);
    let source = HandlerSource::discover(root.path(), &relative).expect("handler source");
    (root, source)
}

fn request(id: &str, method: HttpMethod, body: &str) -> HandlerRequest {
    let mut request = HandlerRequest::route(id, "/probe", method);
    request.body = Some(HandlerBody {
        encoding: HandlerBodyEncoding::Utf8,
        data: String::from(body),
    });
    request
}

#[cfg(unix)]
fn request_with_pid_file(
    id: &str,
    method: HttpMethod,
    body: &str,
    pid_file: &Path,
) -> HandlerRequest {
    let mut request = request(id, method, body);
    request.headers.insert(
        String::from("x-pid-file"),
        vec![pid_file.to_string_lossy().into_owned()],
    );
    request
}

fn ordinary_variable() -> (&'static str, String) {
    ["HOME", "USERPROFILE", "CARGO_HOME", "CI"]
        .iter()
        .find_map(|name| std::env::var(name).ok().map(|value| (*name, value)))
        .expect("test host needs one non-baseline environment variable")
}

fn supervisor_for(root: &Path, options: HandlerSupervisorOptions) -> HandlerSupervisor {
    let application = ApplicationConfig::load(root).expect("application configuration");
    HandlerSupervisor::new(HandlerSupervisorOptions {
        application,
        ..options
    })
    .expect("supervisor")
}

fn timing_options() -> HandlerSupervisorOptions {
    HandlerSupervisorOptions {
        default_timeout: Duration::from_secs(10),
        cancellation_grace: Duration::from_secs(1),
        ..HandlerSupervisorOptions::default()
    }
}

// For tests that cut a relay off at the invocation budget: an 8 s request with a 2 s grace leaves the
// handler 6 s, enough to start and launch its relay on a loaded machine.
const LOADED_REQUEST_DEADLINE_MS: u64 = 8_000;

fn loaded_timing_options() -> HandlerSupervisorOptions {
    HandlerSupervisorOptions {
        cancellation_grace: Duration::from_secs(2),
        ..timing_options()
    }
}

fn body(response: &tachyon_contracts::HandlerResponse) -> serde_json::Value {
    let data = &response.body.as_ref().expect("response body").data;
    serde_json::from_str(data).expect("JSON body")
}

#[cfg(unix)]
fn process_exists(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(unix)]
async fn assert_process_gone(pid_file: &Path) {
    let pid: u32 = fs::read_to_string(pid_file)
        .expect("pid file")
        .trim()
        .parse()
        .expect("pid");
    for _attempt in 0..250 {
        if !process_exists(pid) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("relay descendant {pid} survived the invocation");
}

#[cfg(unix)]
async fn wait_for_file(path: &Path) {
    for _attempt in 0..250 {
        if path.is_file() && fs::metadata(path).is_ok_and(|metadata| metadata.len() > 0) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("relay program did not start");
}

async fn relay_environment(extension: &str, handler: &str, ordinary: (&str, &str)) {
    let (root, source) = project(extension, handler);
    let options = HandlerSupervisorOptions {
        environment: EnvironmentPolicy::from_names([ordinary.0]).expect("allowlist"),
        default_timeout: Duration::from_mins(1),
        ..HandlerSupervisorOptions::default()
    };
    let supervisor = supervisor_for(root.path(), options);
    let body_text = format!("environment {}", ordinary.0);
    let response = supervisor
        .invoke(
            &source,
            &request("relay_environment", HttpMethod::Post, &body_text),
            &HandlerCancellation::default(),
        )
        .await
        .expect("relay response");
    assert_eq!(response.status, 200, "{extension}: {response:?}");
    let seen = body(&response);
    assert_eq!(seen["application"], false, "{extension}: {seen}");
    assert_eq!(
        seen["resources"],
        serde_json::json!([]),
        "{extension}: {seen}"
    );
    assert_eq!(
        seen["path"].as_str(),
        std::env::var("PATH").ok().as_deref(),
        "{extension}"
    );
    assert_eq!(seen["ordinary"], ordinary.1, "{extension}: {seen}");
}

#[tokio::test]
async fn interpreted_handlers_keep_their_capabilities_but_relay_programs_do_not() {
    let (name, value) = ordinary_variable();
    for (extension, handler) in [
        ("js", Handlers::JAVASCRIPT),
        ("py", Handlers::PYTHON),
        ("rb", Handlers::RUBY),
    ] {
        if !runnable(extension) {
            continue;
        }
        let (root, source) = project(extension, handler);
        let supervisor = supervisor_for(root.path(), HandlerSupervisorOptions::default());
        let own = supervisor
            .invoke(
                &source,
                &request("handler_environment", HttpMethod::Get, ""),
                &HandlerCancellation::default(),
            )
            .await
            .expect("handler environment");
        assert_eq!(body(&own)["application"], true, "{extension}");
        assert_eq!(body(&own)["token"], true, "{extension}");
        relay_environment(extension, handler, (name, &value)).await;
    }
}

#[tokio::test]
async fn compiled_relay_programs_receive_only_the_baseline_and_allowlist() {
    let (name, value) = ordinary_variable();
    for extension in ["php", "java", "kt", "cs", "rs", "dart", "go", "hpp"] {
        if !runnable(extension) {
            continue;
        }
        let handler = Handlers::compiled(extension, "environment", name).expect("handler");
        relay_environment(extension, &handler, (name, &value)).await;
    }
}

async fn assert_relay_fails_before(
    supervisor: &HandlerSupervisor,
    source: &HandlerSource,
    request: &HandlerRequest,
    started: Instant,
    limit: Duration,
) {
    let response = supervisor
        .invoke(source, request, &HandlerCancellation::default())
        .await
        .expect("relay failure is a response, not a supervisor timeout");
    assert_eq!(response.status, 502, "{response:?}");
    assert!(started.elapsed() < limit, "{:?}", started.elapsed());
}

// The handler starts for 3 s and the request allows 8 s with a 2 s cancellation grace, so the relay gets
// what is left of a 6 s budget and its 502 arrives at about 6 s. If start-up time did not count, the relay
// would run until about 9 s, past the 8 s deadline, and the supervisor would time out instead.
#[tokio::test]
async fn start_up_time_counts_against_the_relay_deadline() {
    for (extension, handler) in [
        ("js", Handlers::JAVASCRIPT_SLOW_START),
        ("py", Handlers::PYTHON_SLOW_START),
    ] {
        if !runnable(extension) {
            continue;
        }
        let (root, source) = project(extension, handler);
        let supervisor = supervisor_for(root.path(), loaded_timing_options());
        let mut slow = request("slow_start", HttpMethod::Post, "");
        slow.deadline_ms = Some(LOADED_REQUEST_DEADLINE_MS);
        let started = Instant::now();
        let deadline = Duration::from_millis(LOADED_REQUEST_DEADLINE_MS);
        assert_relay_fails_before(&supervisor, &source, &slow, started, deadline).await;
    }
}

#[tokio::test]
async fn php_handler_work_counts_against_the_relay_deadline() {
    if !runnable("php") {
        return;
    }
    let handler = Handlers::compiled("php", "sleep:10", "")
        .expect("handler")
        .replacen(
            "return Yon::relay",
            "usleep(800000);\n        return Yon::relay",
            1,
        );
    let (root, source) = project("php", &handler);
    let supervisor = supervisor_for(root.path(), timing_options());
    let mut slow = request("php_slow_start", HttpMethod::Post, "");
    slow.deadline_ms = Some(3_000);
    assert_relay_fails_before(
        &supervisor,
        &source,
        &slow,
        Instant::now(),
        Duration::from_millis(2_600),
    )
    .await;
}

#[test]
fn php_worker_files_reject_top_level_setup_outside_the_class() {
    for setup in [
        "usleep(800000);",
        "putenv('YON_RELAY_STARTUP_TEST=from-application');",
        "namespace Example;",
    ] {
        let root = tempfile::tempdir().expect("project");
        let relative = "server/workers/probe.php";
        write(
            root.path(),
            relative,
            &format!("<?php\n{setup}\nfinal class Handler {{}}\n"),
        );
        let Err(failure) = HandlerSource::discover(root.path(), relative) else {
            panic!("authored setup was accepted: {setup}");
        };
        assert!(failure.to_string().contains("TY1310"), "{failure}");
    }
}

#[tokio::test]
async fn php_handler_cannot_add_to_the_relay_environment() {
    if !runnable("php") {
        return;
    }
    let handler = Handlers::compiled("php", "environment", "YON_RELAY_STARTUP_TEST")
        .expect("handler")
        .replacen(
            "return Yon::relay",
            "putenv('YON_RELAY_STARTUP_TEST=from-application');\n        return Yon::relay",
            1,
        );
    let (root, source) = project("php", &handler);
    let supervisor = supervisor_for(root.path(), HandlerSupervisorOptions::default());
    let response = supervisor
        .invoke(
            &source,
            &request("php_startup_environment", HttpMethod::Post, ""),
            &HandlerCancellation::default(),
        )
        .await
        .expect("relay response");
    assert_eq!(response.status, 200, "{response:?}");
    assert_eq!(body(&response)["ordinary"], serde_json::Value::Null);
}

#[tokio::test]
async fn php_bootstrap_preserves_class_imports() {
    if !runnable("php") {
        return;
    }
    let handler = r"<?php
use DateTimeImmutable as Clock;

final class Handler {
    public static function GET(): array {
        return ['year' => (new Clock('2026-09-25'))->format('Y')];
    }
}
";
    let (root, source) = project("php", handler);
    let supervisor = supervisor_for(root.path(), HandlerSupervisorOptions::default());
    let response = supervisor
        .invoke(
            &source,
            &request("php_preamble", HttpMethod::Get, ""),
            &HandlerCancellation::default(),
        )
        .await
        .expect("PHP preamble response");
    assert_eq!(response.status, 200, "{response:?}");
    assert_eq!(body(&response)["year"], "2026");
}

#[tokio::test]
async fn queue_time_counts_against_the_relay_deadline() {
    if !runnable("js") {
        return;
    }
    let (root, source) = project("js", Handlers::JAVASCRIPT);
    let (_busy_root, busy_source) = project("js", Handlers::JAVASCRIPT_BUSY);
    let supervisor = supervisor_for(
        root.path(),
        HandlerSupervisorOptions {
            max_concurrency: 1,
            ..timing_options()
        },
    );
    let busy_supervisor = supervisor.clone();
    let busy = tokio::spawn(async move {
        busy_supervisor
            .invoke(
                &busy_source,
                &request("busy", HttpMethod::Get, ""),
                &HandlerCancellation::default(),
            )
            .await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let started = Instant::now();
    let mut queued = request("queued_relay", HttpMethod::Post, "sleep:10");
    queued.deadline_ms = Some(3_000);
    assert_relay_fails_before(
        &supervisor,
        &source,
        &queued,
        started,
        Duration::from_millis(2_800),
    )
    .await;
    busy.await.expect("busy task").expect("busy response");
}

// The first request starts the warm handler and its relay answers at once, so the timed relay measures
// only the default deadline. start_up_time_counts_against_the_relay_deadline covers start-up time.
#[tokio::test]
async fn the_supervisor_default_timeout_limits_relays_without_a_request_deadline() {
    for extension in [
        "js", "py", "rb", "php", "java", "kt", "cs", "rs", "dart", "go", "hpp",
    ] {
        if !runnable(extension) {
            continue;
        }
        let control = tempfile::tempdir().expect("sleep control");
        let sleep_file = control.path().join(Delegate::SLEEP_FILE);
        // Forward slashes need no escaping in the compiled handlers' string literals, and Windows accepts them.
        let sleep_path = sleep_file.to_string_lossy().replace('\\', "/");
        let mode = "sleep-as-written";
        let body = format!("{mode} {sleep_path}");
        let handler = match extension {
            "js" => String::from(Handlers::JAVASCRIPT),
            "py" => String::from(Handlers::PYTHON),
            "rb" => String::from(Handlers::RUBY),
            _ => Handlers::compiled(extension, mode, &sleep_path).expect("handler"),
        };
        let (root, source) = project(extension, &handler);
        let supervisor = supervisor_for(
            root.path(),
            HandlerSupervisorOptions {
                default_timeout: Duration::from_secs(8),
                cancellation_grace: Duration::from_secs(2),
                ..HandlerSupervisorOptions::default()
            },
        );
        fs::write(&sleep_file, "0").expect("sleep seconds");
        let warm = supervisor
            .invoke(
                &source,
                &request("start_handler", HttpMethod::Post, &body),
                &HandlerCancellation::default(),
            )
            .await
            .expect("the handler starts within the default timeout");
        assert_eq!(warm.status, 200, "{extension}: {warm:?}");
        fs::write(&sleep_file, "30").expect("sleep seconds");
        let unbounded = request("default_deadline", HttpMethod::Post, &body);
        let started = Instant::now();
        assert_relay_fails_before(
            &supervisor,
            &source,
            &unbounded,
            started,
            Duration::from_secs(8),
        )
        .await;
    }
}

#[tokio::test]
async fn sequential_relays_and_a_longer_request_deadline_share_one_budget() {
    for (extension, handler) in [
        ("js", Handlers::JAVASCRIPT),
        ("py", Handlers::PYTHON),
        ("rb", Handlers::RUBY),
        ("hpp", Handlers::CPP),
    ] {
        if !runnable(extension) {
            continue;
        }
        let (root, source) = project(extension, handler);
        let supervisor = supervisor_for(root.path(), timing_options());
        let mut twice = request("twice", HttpMethod::Post, "twice");
        twice.deadline_ms = Some(3_500);
        let response = supervisor
            .invoke(&source, &twice, &HandlerCancellation::default())
            .await
            .expect("both relays answer within the invocation");
        let statuses = body(&response);
        assert_eq!(statuses["first"], 200, "{extension}: {statuses}");
        assert_eq!(statuses["second"], 502, "{extension}: {statuses}");
    }
}

#[tokio::test]
async fn a_relay_program_that_never_reads_its_input_is_bounded() {
    for (extension, handler) in [
        ("js", Handlers::JAVASCRIPT),
        ("py", Handlers::PYTHON),
        ("rb", Handlers::RUBY),
        ("hpp", Handlers::CPP),
    ] {
        if !runnable(extension) {
            continue;
        }
        let (root, source) = project(extension, handler);
        let supervisor = supervisor_for(root.path(), loaded_timing_options());
        let padding = vec!["x".repeat(8_000); 64];
        let mut large = request("ignored_input", HttpMethod::Post, "ignore-input");
        large.headers.insert(String::from("x-padding"), padding);
        large.deadline_ms = Some(LOADED_REQUEST_DEADLINE_MS);
        let started = Instant::now();
        // The relay's 502 must arrive within the request deadline.
        let deadline = Duration::from_millis(LOADED_REQUEST_DEADLINE_MS);
        assert_relay_fails_before(&supervisor, &source, &large, started, deadline).await;
    }
}

#[cfg(unix)]
#[tokio::test]
async fn a_descendant_holding_the_relay_pipes_is_bounded_and_reaped() {
    for (extension, handler) in [
        ("js", Handlers::JAVASCRIPT),
        ("py", Handlers::PYTHON),
        ("rb", Handlers::RUBY),
        ("hpp", Handlers::CPP),
    ] {
        if !runnable(extension) {
            continue;
        }
        let (root, source) = project(extension, handler);
        let supervisor = supervisor_for(root.path(), loaded_timing_options());
        let pid_file = root.path().join("inherit.pid");
        let mut inherit = request_with_pid_file("inherit", HttpMethod::Post, "inherit", &pid_file);
        inherit.deadline_ms = Some(LOADED_REQUEST_DEADLINE_MS);
        let started = Instant::now();
        // The relay's 502 must arrive within the request deadline.
        let deadline = Duration::from_millis(LOADED_REQUEST_DEADLINE_MS);
        assert_relay_fails_before(&supervisor, &source, &inherit, started, deadline).await;
        assert_process_gone(&pid_file).await;
    }
}

#[cfg(unix)]
#[tokio::test]
async fn cancelling_an_invocation_reaps_its_relay_program() {
    for (extension, handler) in [
        ("js", Handlers::JAVASCRIPT),
        ("py", Handlers::PYTHON),
        ("rb", Handlers::RUBY),
        ("hpp", Handlers::CPP),
    ] {
        if !runnable(extension) {
            continue;
        }
        let (root, source) = project(extension, handler);
        let supervisor = supervisor_for(root.path(), timing_options());
        let pid_file = root.path().join("hold.pid");
        let hold = request_with_pid_file("hold", HttpMethod::Post, "hold", &pid_file);
        let cancellation = HandlerCancellation::default();
        let cancel = cancellation.clone();
        let watched = pid_file.clone();
        let canceller = tokio::spawn(async move {
            wait_for_file(&watched).await;
            cancel.cancel();
        });
        let failure = supervisor
            .invoke(&source, &hold, &cancellation)
            .await
            .expect_err("cancelled invocation");
        assert!(failure.to_string().contains("TY2111"), "{failure}");
        canceller.await.expect("canceller");
        assert_process_gone(&pid_file).await;
    }
}

#[cfg(unix)]
#[tokio::test]
async fn closing_a_stream_reaps_its_relay_program() {
    if !runnable("js") {
        return;
    }
    let (root, source) = project("js", Handlers::JAVASCRIPT_STREAM);
    let supervisor = supervisor_for(root.path(), timing_options());
    let pid_file = root.path().join("stream.pid");
    let stream = request_with_pid_file("stream", HttpMethod::Get, "", &pid_file);
    let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
    let invocation = tokio::spawn({
        let supervisor = supervisor.clone();
        async move { supervisor.invoke_streaming(&source, &stream, sender).await }
    });
    let first = receiver.recv().await.expect("first event");
    assert_eq!(first.data, "\"started\"");
    wait_for_file(&pid_file).await;
    drop(receiver);
    invocation
        .await
        .expect("stream task")
        .expect("closed stream ends cleanly");
    assert_process_gone(&pid_file).await;
}

struct RelayCase;

impl RelayCase {
    const FIXED_FAILURE: &str = "Component invocation failed.";
    const MISSING_PROGRAM: &str = "tachyon-missing-relay-program";
    const DYNAMIC: [(&str, &str); 5] = [
        ("js", "node"),
        ("js", "bun"),
        ("py", "python3"),
        ("rb", "ruby"),
        ("hpp", "clang++"),
    ];
    const COMPILED: [&str; 8] = ["php", "java", "kt", "cs", "rs", "dart", "go", "hpp"];

    fn dynamic_handler(extension: &str) -> &'static str {
        match extension {
            "py" => Handlers::PYTHON,
            "rb" => Handlers::RUBY,
            "hpp" => Handlers::CPP,
            _ => Handlers::JAVASCRIPT,
        }
    }

    fn options(runtime: &str) -> HandlerSupervisorOptions {
        let mut options = timing_options();
        if runtime == "bun" {
            options.runtimes.javascript = "bun".into();
        }
        options
    }
}

async fn assert_fixed_relay_failure(
    extension: &str,
    handler: &str,
    options: HandlerSupervisorOptions,
    body_text: &str,
) {
    let (root, source) = project(extension, handler);
    let supervisor = supervisor_for(root.path(), options);
    let mut relayed = request("fixed_failure", HttpMethod::Post, body_text);
    relayed.deadline_ms = Some(8_000);
    let response = supervisor
        .invoke(&source, &relayed, &HandlerCancellation::default())
        .await
        .expect("relay failure is a response");
    assert_eq!(
        response.status, 502,
        "{extension} {body_text}: {response:?}"
    );
    assert_eq!(
        body(&response),
        serde_json::json!({"error": RelayCase::FIXED_FAILURE}),
        "{extension} {body_text}"
    );
}

#[tokio::test]
async fn dynamic_relay_failures_reach_the_handler_as_one_fixed_message() {
    for (extension, runtime) in RelayCase::DYNAMIC {
        if !runnable(extension) || !available(runtime) {
            continue;
        }
        let handler = RelayCase::dynamic_handler(extension);
        for body_text in ["print null", "print [1]", "print not-json", "missing"] {
            let options = RelayCase::options(runtime);
            assert_fixed_relay_failure(extension, handler, options, body_text).await;
        }
    }
}

#[tokio::test]
async fn compiled_relay_failures_reach_the_handler_as_one_fixed_message() {
    for extension in RelayCase::COMPILED {
        if !runnable(extension) {
            continue;
        }
        let not_an_object = Handlers::compiled(extension, "print", "[1]").expect("handler");
        let missing =
            Handlers::compiled_with_program(extension, RelayCase::MISSING_PROGRAM, "", "")
                .expect("handler");
        for handler in [not_an_object, missing] {
            assert_fixed_relay_failure(extension, &handler, timing_options(), "").await;
        }
    }
}

#[tokio::test]
async fn a_missing_relay_program_fails_without_waiting_for_the_deadline() {
    for (extension, runtime) in RelayCase::DYNAMIC {
        if !runnable(extension) || !available(runtime) {
            continue;
        }
        let handler = RelayCase::dynamic_handler(extension);
        let (root, source) = project(extension, handler);
        let supervisor = supervisor_for(root.path(), RelayCase::options(runtime));
        let mut missing = request("missing_program", HttpMethod::Post, "missing");
        missing.deadline_ms = Some(9_000);
        let started = Instant::now();
        assert_relay_fails_before(
            &supervisor,
            &source,
            &missing,
            started,
            Duration::from_secs(4),
        )
        .await;
    }
}

#[cfg(unix)]
#[tokio::test]
async fn a_php_relay_program_cannot_write_to_descriptors_the_handler_opened() {
    if !runnable("php") {
        return;
    }
    let handler = Handlers::compiled("php", "forge", "")
        .expect("handler")
        .replacen(
            "return Yon::relay",
            "$log = fopen('php://stdout', 'w');\n        $listener = stream_socket_server('tcp://127.0.0.1:0');\n        return Yon::relay",
            1,
        );
    let (root, source) = project("php", &handler);
    let supervisor = supervisor_for(root.path(), HandlerSupervisorOptions::default());
    let response = supervisor
        .invoke(
            &source,
            &request("php_descriptors", HttpMethod::Post, ""),
            &HandlerCancellation::default(),
        )
        .await
        .expect("genuine relay response");
    assert_eq!(response.status, 200, "{response:?}");
    assert_eq!(body(&response), serde_json::json!({"genuine": true}));
}

#[cfg(unix)]
#[tokio::test]
async fn a_go_relay_program_cannot_write_to_descriptors_the_handler_opened() {
    if !runnable("go") {
        return;
    }
    let handler = Handlers::compiled("go", "forge", "")
        .expect("handler")
        .replacen(
            "type Handler",
            "import (\n\t\"net\"\n\t\"os\"\n)\n\ntype Handler",
            1,
        )
        .replacen(
            "\treturn Relay",
            "\tlog, _ := os.OpenFile(\"/dev/stdout\", os.O_WRONLY, 0)\n\tdefer log.Close()\n\tlistener, _ := net.Listen(\"tcp\", \"127.0.0.1:0\")\n\tdefer listener.Close()\n\treturn Relay",
            1,
        );
    let (root, source) = project("go", &handler);
    let supervisor = supervisor_for(root.path(), HandlerSupervisorOptions::default());
    let response = supervisor
        .invoke(
            &source,
            &request("go_descriptors", HttpMethod::Post, ""),
            &HandlerCancellation::default(),
        )
        .await
        .expect("genuine relay response");
    assert_eq!(response.status, 200, "{response:?}");
    assert_eq!(body(&response), serde_json::json!({"genuine": true}));
}

#[tokio::test]
async fn php_warnings_never_reach_the_response_stream() {
    if !runnable("php") {
        return;
    }
    let handler = r"<?php
final class Handler {
    public static function GET(): array {
        trigger_error('warning-canary', E_USER_WARNING);
        return ['answered' => true];
    }
}
";
    let (root, source) = project("php", handler);
    let supervisor = supervisor_for(root.path(), HandlerSupervisorOptions::default());
    let response = supervisor
        .invoke(
            &source,
            &request("php_warning", HttpMethod::Get, ""),
            &HandlerCancellation::default(),
        )
        .await
        .expect("PHP response despite a warning");
    assert_eq!(response.status, 200, "{response:?}");
    assert_eq!(body(&response), serde_json::json!({"answered": true}));
}

struct DeadlineTampering;

impl DeadlineTampering {
    const PYTHON: &str = r#"
class Handler:
    @staticmethod
    def GET(request):
        attempts = []
        for holder in (YonGateway, relay.__self__):
            try:
                holder.deadline = 10 ** 12
                attempts.append("changed")
            except AttributeError:
                attempts.append("refused")
        return {"attempts": attempts}
"#;

    const PHP: &str = r"<?php
final class Handler {
    public static function GET(): array {
        try {
            YonRelayContext::captureInvocation(['deadline_ms' => 1e12]);
            return ['attempts' => ['changed']];
        } catch (Error $refused) {
            return ['attempts' => ['refused']];
        }
    }
}
";

    const GO: &str = r#"import (
	"time"
	"yonhandler/yoninvocation"
)

type Handler struct{}

func (Handler) GET() any {
	yoninvocation.current.deadline = time.Now().Add(time.Hour)
	return yoninvocation.TimeLeft()
}
"#;

    const DART: &str = r"class Handler {
  Object? GET() {
    try {
      _YonRelayState._deadlineMilliseconds = 1 << 40;
      return {'attempts': ['changed']};
    } catch (_) {
      return {'attempts': ['refused']};
    }
  }
}
";
}

#[tokio::test]
async fn handler_code_cannot_extend_its_relay_deadline() {
    for (extension, handler, refusals) in [
        ("py", DeadlineTampering::PYTHON, 2),
        ("php", DeadlineTampering::PHP, 1),
        ("dart", DeadlineTampering::DART, 1),
    ] {
        if !runnable(extension) {
            continue;
        }
        let (root, source) = project(extension, handler);
        let supervisor = supervisor_for(root.path(), HandlerSupervisorOptions::default());
        let response = supervisor
            .invoke(
                &source,
                &request("tampering", HttpMethod::Get, ""),
                &HandlerCancellation::default(),
            )
            .await
            .expect("tampering response");
        assert_eq!(response.status, 200, "{extension}: {response:?}");
        let refused = vec!["refused"; refusals];
        assert_eq!(
            body(&response)["attempts"],
            serde_json::json!(refused),
            "{extension}"
        );
    }
}

#[tokio::test]
async fn go_handler_code_cannot_reach_its_relay_deadline() {
    if !runnable("go") {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let relative = "server/workers/probe.go";
    write(root.path(), relative, DeadlineTampering::GO);
    let failure = HandlerSource::discover(root.path(), relative)
        .expect_err("the relay deadline is not reachable from handler code");
    let message = failure.to_string();
    assert!(message.contains("current not exported"), "{message}");
}

#[tokio::test]
async fn csharp_work_before_main_counts_against_the_relay_deadline() {
    if !runnable("cs") {
        return;
    }
    let handler = Handlers::compiled("cs", "sleep:10", "")
        .expect("handler")
        .replacen(
            "    public static HttpResponseMessage POST",
            "    [System.Runtime.CompilerServices.ModuleInitializer]\n    internal static void Delay() => System.Threading.Thread.Sleep(800);\n    public static HttpResponseMessage POST",
            1,
        );
    let (root, source) = project("cs", &handler);
    let supervisor = supervisor_for(root.path(), timing_options());
    let mut slow = request("csharp_slow_start", HttpMethod::Post, "");
    slow.deadline_ms = Some(3_000);
    assert_relay_fails_before(
        &supervisor,
        &source,
        &slow,
        Instant::now(),
        Duration::from_millis(2_600),
    )
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_cpp_relay_program_cannot_write_to_descriptors_the_handler_opened() {
    if !runnable("hpp") {
        return;
    }
    let handler = Handlers::compiled("hpp", "forge", "")
        .expect("handler")
        .replacen("#pragma once\n", "#pragma once\n#include <cstdio>\n", 1)
        .replacen(
            "        return relay",
            "        std::FILE* log = std::fopen(\"/dev/stdout\", \"w\");\n        tachyon::http::HttpResponseMessage answer = relay",
            1,
        )
        .replacen(
            "request);\n    }",
            "request);\n        std::fclose(log);\n        return answer;\n    }",
            1,
        );
    let (root, source) = project("hpp", &handler);
    let supervisor = supervisor_for(root.path(), HandlerSupervisorOptions::default());
    let response = supervisor
        .invoke(
            &source,
            &request("cpp_descriptors", HttpMethod::Post, ""),
            &HandlerCancellation::default(),
        )
        .await
        .expect("genuine relay response");
    assert_eq!(response.status, 200, "{response:?}");
    assert_eq!(body(&response), serde_json::json!({"genuine": true}));
}

#[tokio::test]
async fn cpp_work_before_the_relay_counts_against_its_deadline() {
    if !runnable("hpp") {
        return;
    }
    let handler = Handlers::compiled("hpp", "sleep:10", "")
        .expect("handler")
        .replacen("#pragma once\n", "#pragma once\n#include <chrono>\n#include <thread>\n", 1)
        .replacen(
            "        return relay",
            "        std::this_thread::sleep_for(std::chrono::milliseconds(800));\n        return relay",
            1,
        );
    let (root, source) = project("hpp", &handler);
    let supervisor = supervisor_for(root.path(), timing_options());
    let mut slow = request("cpp_slow_start", HttpMethod::Post, "");
    slow.deadline_ms = Some(3_000);
    assert_relay_fails_before(
        &supervisor,
        &source,
        &slow,
        Instant::now(),
        Duration::from_millis(2_600),
    )
    .await;
}
