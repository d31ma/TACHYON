#![allow(clippy::expect_used)]

use std::fs;
use std::time::Duration;
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HandlerResponse, HttpMethod,
};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions, WarmSettings,
    YonIsolationPolicy,
};

struct WarmKotlin;

impl WarmKotlin {
    const COUNTER_PATH: &str = "server/workers/warm.kt";
    const COUNTER: &str = r#"
object Handler {
    private var count = 0

    @JvmStatic fun POST(exchange: com.sun.net.httpserver.HttpExchange): Any? {
        val mode = exchange.requestBody.readAllBytes().decodeToString()
        if (mode == "crash") kotlin.system.exitProcess(23)
        count += 1
        return mapOf("pid" to ProcessHandle.current().pid(), "count" to count, "mode" to mode)
    }
}
"#;
    const STREAM_PATH: &str = "server/workers/beats.kt";
    const STREAM: &str = r#"
object Handler {
    @JvmStatic fun POST(): Sequence<Any?> = sequence {
        for (index in 0 until 3) {
            yield(mapOf("pid" to ProcessHandle.current().pid(), "index" to index))
        }
    }
}
"#;

    fn kotlin_usable() -> bool {
        let found = crate::support::program_probe::ProgramProbe::answers("kotlinc", "-version");
        let usable = crate::support::toolchain::Toolchain::checked("kotlinc", found);
        if !usable {
            eprintln!("skipping Kotlin warm handler execution: kotlinc is unavailable");
        }
        usable
    }

    fn source(root: &tempfile::TempDir, relative: &str, contents: &str) -> HandlerSource {
        let path = root.path().join(relative);
        fs::create_dir_all(path.parent().expect("handler parent")).expect("handler directory");
        fs::write(&path, contents).expect("handler source");
        HandlerSource::discover(root.path(), relative).expect("validated source")
    }

    fn counter(root: &tempfile::TempDir) -> HandlerSource {
        Self::source(root, Self::COUNTER_PATH, Self::COUNTER)
    }

    fn stream(root: &tempfile::TempDir) -> HandlerSource {
        Self::source(root, Self::STREAM_PATH, Self::STREAM)
    }

    fn request(id: &str, mode: &str) -> HandlerRequest {
        let mut request = HandlerRequest::route(id, "/warm", HttpMethod::Post);
        request.body = Some(HandlerBody {
            encoding: HandlerBodyEncoding::Utf8,
            data: String::from(mode),
        });
        request
    }

    fn supervisor(isolation: YonIsolationPolicy) -> HandlerSupervisor {
        HandlerSupervisor::new(HandlerSupervisorOptions {
            isolation,
            default_timeout: Duration::from_mins(1),
            ..HandlerSupervisorOptions::default()
        })
        .expect("supervisor")
    }

    fn warm() -> HandlerSupervisor {
        let settings = WarmSettings::new(1_000, Duration::from_mins(1));
        Self::supervisor(YonIsolationPolicy::Warm(settings))
    }

    fn body_json(response: &HandlerResponse) -> serde_json::Value {
        let body = response.body.as_ref().expect("successful body");
        serde_json::from_str(&body.data).expect("JSON handler result")
    }

    async fn answer(
        supervisor: &HandlerSupervisor,
        source: &HandlerSource,
        id: &str,
        mode: &str,
    ) -> (u64, u64) {
        let response = supervisor
            .invoke(
                source,
                &Self::request(id, mode),
                &HandlerCancellation::default(),
            )
            .await
            .expect("Kotlin response");
        let value = Self::body_json(&response);
        assert_eq!(value["mode"], mode, "{id} read its own request body");
        let pid = value["pid"].as_u64().expect("pid");
        let count = value["count"].as_u64().expect("count");
        (pid, count)
    }

    async fn stream_pid(supervisor: &HandlerSupervisor, source: &HandlerSource, id: &str) -> u64 {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
        let request = Self::request(id, "");
        let invocation = supervisor.invoke_streaming(source, &request, sender);
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
        assert_eq!(first["index"], 0, "{id}");
        first["pid"].as_u64().expect("pid")
    }
}

#[tokio::test]
async fn a_warm_kotlin_handler_serves_later_requests_from_the_same_process() {
    if !WarmKotlin::kotlin_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = WarmKotlin::counter(&root);
    let supervisor = WarmKotlin::warm();
    let (first_pid, first_count) =
        WarmKotlin::answer(&supervisor, &source, "kotlin_first", "one").await;
    let (second_pid, second_count) =
        WarmKotlin::answer(&supervisor, &source, "kotlin_second", "two").await;
    assert_eq!(first_pid, second_pid, "the Kotlin process was reused");
    assert_eq!(first_count, 1);
    assert_eq!(second_count, 2, "the Kotlin process kept its object state");
}

#[tokio::test]
async fn a_kotlin_handler_in_process_mode_starts_a_new_process_for_every_request() {
    if !WarmKotlin::kotlin_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = WarmKotlin::counter(&root);
    let supervisor = WarmKotlin::supervisor(YonIsolationPolicy::Process);
    let (first_pid, first_count) =
        WarmKotlin::answer(&supervisor, &source, "kotlin_process_1", "one").await;
    let (second_pid, second_count) =
        WarmKotlin::answer(&supervisor, &source, "kotlin_process_2", "two").await;
    assert_ne!(first_pid, second_pid);
    assert_eq!([first_count, second_count], [1, 1]);
}

#[tokio::test]
async fn a_kotlin_crash_replaces_the_warm_process() {
    if !WarmKotlin::kotlin_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = WarmKotlin::counter(&root);
    let supervisor = WarmKotlin::warm();
    let (before, _) = WarmKotlin::answer(&supervisor, &source, "kotlin_before", "one").await;
    let crashed = supervisor
        .invoke(
            &source,
            &WarmKotlin::request("kotlin_crash", "crash"),
            &HandlerCancellation::default(),
        )
        .await
        .expect_err("crash");
    assert!(crashed.to_string().contains("TY2104"), "{crashed}");
    let (after, count) = WarmKotlin::answer(&supervisor, &source, "kotlin_after", "two").await;
    assert_ne!(before, after, "a new Kotlin process started");
    assert_eq!(count, 1, "the new Kotlin process started with fresh state");
}

#[tokio::test]
async fn a_warm_kotlin_stream_ends_with_its_end_frame_and_stays_warm() {
    if !WarmKotlin::kotlin_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = WarmKotlin::stream(&root);
    let supervisor = WarmKotlin::warm();
    let first = WarmKotlin::stream_pid(&supervisor, &source, "kotlin_stream_first").await;
    let second = WarmKotlin::stream_pid(&supervisor, &source, "kotlin_stream_second").await;
    assert_eq!(first, second, "the Kotlin stream process stayed warm");
}
