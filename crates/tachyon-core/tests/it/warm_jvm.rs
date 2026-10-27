#![allow(clippy::expect_used)]

use crate::support::program_probe::ProgramProbe;
use crate::support::toolchain::Toolchain;
use std::fs;
use std::time::Duration;
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HandlerResponse, HttpMethod,
};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions, WarmSettings,
    YonIsolationPolicy,
};

struct JvmHandler {
    extension: &'static str,
    compiler: &'static str,
    version_flag: &'static str,
    counter: &'static str,
    stream: &'static str,
}

impl JvmHandler {
    const JAVA: Self = Self {
        extension: "java",
        compiler: "javac",
        version_flag: "-version",
        counter: r#"
class Handler {
    static long count = 0;

    static void POST(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        String mode = new String(exchange.getRequestBody().readAllBytes(), java.nio.charset.StandardCharsets.UTF_8);
        if (mode.equals("crash")) System.exit(23);
        count += 1;
        String json = "{\"pid\":" + ProcessHandle.current().pid() + ",\"count\":" + count + "}";
        byte[] body = json.getBytes(java.nio.charset.StandardCharsets.UTF_8);
        exchange.getResponseHeaders().add("content-type", "application/json");
        exchange.sendResponseHeaders(200, body.length);
        exchange.getResponseBody().write(body);
    }
}
"#,
        stream: r#"
class Handler {
    static java.util.stream.Stream<Object> POST() {
        long pid = ProcessHandle.current().pid();
        return java.util.stream.IntStream.range(0, 3)
            .<Object>mapToObj(index -> java.util.Map.of("pid", pid, "index", index));
    }
}
"#,
    };

    const GROOVY: Self = Self {
        extension: "groovy",
        compiler: "groovyc",
        version_flag: "--version",
        counter: r#"
import com.sun.net.httpserver.HttpExchange

class Handler {
    static long count = 0

    static void POST(HttpExchange exchange) {
        String mode = new String(exchange.requestBody.readAllBytes(), 'UTF-8')
        if (mode == 'crash') System.exit(23)
        count += 1
        String json = '{"pid":' + ProcessHandle.current().pid() + ',"count":' + count + '}'
        byte[] body = json.getBytes('UTF-8')
        exchange.responseHeaders.add('content-type', 'application/json')
        exchange.sendResponseHeaders(200, body.length)
        exchange.responseBody.write(body)
    }
}
"#,
        stream: r"
class Handler {
    static Iterator<Object> POST() {
        long pid = ProcessHandle.current().pid()
        return (0..2).collect { index -> [pid: pid, index: index] }.iterator()
    }
}
",
    };

    const ALL: [Self; 2] = [Self::JAVA, Self::GROOVY];

    fn available(&self) -> bool {
        let found = ProgramProbe::answers(self.compiler, self.version_flag);
        if !found {
            eprintln!(
                "skipping warm {}: {} is unavailable",
                self.extension, self.compiler
            );
        }
        Toolchain::checked(self.compiler, found)
    }

    fn source(&self, root: &tempfile::TempDir, contents: &str) -> HandlerSource {
        let relative = format!("server/workers/warm.{}", self.extension);
        let path = root.path().join(&relative);
        fs::create_dir_all(path.parent().expect("handler parent")).expect("handler directory");
        fs::write(path, contents).expect("handler source");
        HandlerSource::discover(root.path(), &relative).expect("validated source")
    }
}

fn request(id: &str, mode: &str) -> HandlerRequest {
    let mut request = HandlerRequest::route(id, "/warm", HttpMethod::Post);
    request.body = Some(HandlerBody {
        encoding: HandlerBodyEncoding::Utf8,
        data: String::from(mode),
    });
    request
}

fn warm() -> HandlerSupervisor {
    let settings = WarmSettings::new(1_000, Duration::from_mins(1));
    HandlerSupervisor::new(HandlerSupervisorOptions {
        isolation: YonIsolationPolicy::Warm(settings),
        default_timeout: Duration::from_mins(2),
        ..HandlerSupervisorOptions::default()
    })
    .expect("warm supervisor")
}

fn body_json(response: &HandlerResponse) -> serde_json::Value {
    let body = response.body.as_ref().expect("successful body");
    serde_json::from_str(&body.data).expect("JSON handler result")
}

async fn answer(supervisor: &HandlerSupervisor, source: &HandlerSource, id: &str) -> (u64, u64) {
    let response = supervisor
        .invoke(source, &request(id, ""), &HandlerCancellation::default())
        .await
        .expect("warm response");
    let value = body_json(&response);
    let pid = value["pid"].as_u64().expect("pid");
    let count = value["count"].as_u64().expect("count");
    (pid, count)
}

async fn stream_pid(supervisor: &HandlerSupervisor, source: &HandlerSource, id: &str) -> u64 {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
    let streamed = request(id, "");
    let invocation = supervisor.invoke_streaming(source, &streamed, sender);
    let collect = async {
        let mut events = Vec::new();
        while let Some(event) = receiver.recv().await {
            events.push(event);
        }
        events
    };
    let (result, events) = tokio::join!(invocation, collect);
    result.expect("stream finished");
    assert_eq!(events.len(), 3, "{id}");
    let first: serde_json::Value = serde_json::from_str(&events[0].data).expect("event");
    first["pid"].as_u64().expect("pid")
}

#[tokio::test]
async fn a_warm_jvm_handler_serves_later_requests_from_the_same_process() {
    for handler in JvmHandler::ALL {
        if !handler.available() {
            continue;
        }
        let root = tempfile::tempdir().expect("project");
        let source = handler.source(&root, handler.counter);
        let supervisor = warm();
        let (first_pid, first_count) = answer(&supervisor, &source, "warm_first").await;
        let (second_pid, second_count) = answer(&supervisor, &source, "warm_second").await;
        let extension = handler.extension;
        assert_eq!(first_pid, second_pid, "{extension} reused its process");
        assert_eq!(first_count, 1, "{extension}");
        assert_eq!(second_count, 2, "{extension} kept its class state");
    }
}

#[tokio::test]
async fn a_crash_replaces_the_warm_jvm_process() {
    for handler in JvmHandler::ALL {
        if !handler.available() {
            continue;
        }
        let root = tempfile::tempdir().expect("project");
        let source = handler.source(&root, handler.counter);
        let supervisor = warm();
        let (before, _) = answer(&supervisor, &source, "before_crash").await;
        let crashed = supervisor
            .invoke(
                &source,
                &request("crash", "crash"),
                &HandlerCancellation::default(),
            )
            .await
            .expect_err("crash");
        assert!(crashed.to_string().contains("TY2104"), "{crashed}");
        let (after, count) = answer(&supervisor, &source, "after_crash").await;
        let extension = handler.extension;
        assert_ne!(before, after, "{extension} started a new process");
        assert_eq!(count, 1, "{extension} started with fresh state");
    }
}

#[tokio::test]
async fn a_warm_jvm_stream_ends_with_its_end_frame_and_stays_warm() {
    for handler in JvmHandler::ALL {
        if !handler.available() {
            continue;
        }
        let root = tempfile::tempdir().expect("project");
        let source = handler.source(&root, handler.stream);
        let supervisor = warm();
        let first = stream_pid(&supervisor, &source, "stream_first").await;
        let second = stream_pid(&supervisor, &source, "stream_second").await;
        assert_eq!(first, second, "{} stayed warm", handler.extension);
    }
}
