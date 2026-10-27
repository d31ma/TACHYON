#![allow(clippy::expect_used)]

use std::fs;
use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HandlerResponse, HttpMethod,
};
use tachyon_core::{
    Failure, HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
    WarmSettings, YonIsolationPolicy,
};

struct WarmRust;

impl WarmRust {
    const COUNTER: &str = r#"
struct Handler;
impl Handler {
    fn POST(request: &__YonFrame) -> Json {
        static COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        if request.body() == "crash" {
            std::process::exit(23);
        }
        let count = COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        Json::map([("pid", u64::from(std::process::id())), ("count", count)])
    }
}
"#;

    const STREAM: &str = r#"
struct Handler;
impl Handler {
    fn POST(request: &__YonFrame) -> __YonAnswer {
        let pid = u64::from(std::process::id());
        if request.body() == "fail" {
            let items: Vec<Result<Json, &str>> = vec![Ok(Json::map([("pid", pid)])), Err("broken")];
            return YonSocket::stream_failing(request, items);
        }
        let items = (0..3_u64).map(move |index| Json::map([("pid", pid), ("index", index)]));
        YonSocket::stream(request, items)
    }
}
"#;

    fn cargo_available() -> bool {
        let found = Command::new("cargo")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if !found {
            eprintln!("skipping warm Rust handlers: cargo is unavailable");
        }
        found
    }

    fn source(root: &tempfile::TempDir, contents: &str) -> HandlerSource {
        let relative = "server/workers/warm.rs";
        let path = root.path().join(relative);
        fs::create_dir_all(path.parent().expect("handler parent")).expect("handler directory");
        fs::write(path, contents).expect("handler source");
        HandlerSource::discover(root.path(), relative).expect("validated source")
    }

    fn request(id: &str, mode: &str) -> HandlerRequest {
        let mut request = HandlerRequest::route(id, "/warm", HttpMethod::Post);
        request.body = Some(HandlerBody {
            encoding: HandlerBodyEncoding::Utf8,
            data: String::from(mode),
        });
        request
    }

    fn supervisor() -> HandlerSupervisor {
        HandlerSupervisor::new(HandlerSupervisorOptions {
            isolation: YonIsolationPolicy::Warm(WarmSettings::new(1_000, Duration::from_secs(60))),
            default_timeout: Duration::from_secs(10),
            ..HandlerSupervisorOptions::default()
        })
        .expect("warm supervisor")
    }

    fn body_json(response: &HandlerResponse) -> serde_json::Value {
        let body = response.body.as_ref().expect("successful body");
        serde_json::from_str(&body.data).expect("JSON handler result")
    }

    async fn answer(
        supervisor: &HandlerSupervisor,
        source: &HandlerSource,
        id: &str,
    ) -> (u64, u64) {
        let response = supervisor
            .invoke(source, &Self::request(id, ""), &HandlerCancellation::default())
            .await
            .expect("warm response");
        let value = Self::body_json(&response);
        let pid = value["pid"].as_u64().expect("pid");
        let count = value["count"].as_u64().expect("count");
        (pid, count)
    }

    async fn stream(
        supervisor: &HandlerSupervisor,
        source: &HandlerSource,
        request: &HandlerRequest,
    ) -> (Result<(), Failure>, Vec<String>) {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
        let invocation = supervisor.invoke_streaming(source, request, sender);
        let collect = async {
            let mut events = Vec::new();
            while let Some(event) = receiver.recv().await {
                events.push(event.data);
            }
            events
        };
        tokio::join!(invocation, collect)
    }

    fn first_pid(events: &[String]) -> u64 {
        let first: serde_json::Value = serde_json::from_str(&events[0]).expect("event");
        first["pid"].as_u64().expect("pid")
    }
}

#[tokio::test]
async fn a_warm_rust_handler_serves_later_requests_from_the_same_process() {
    if !WarmRust::cargo_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = WarmRust::source(&root, WarmRust::COUNTER);
    let supervisor = WarmRust::supervisor();
    let (first_pid, first_count) = WarmRust::answer(&supervisor, &source, "rust_first").await;
    let (second_pid, second_count) = WarmRust::answer(&supervisor, &source, "rust_second").await;
    assert_eq!(first_pid, second_pid, "the Rust handler reused its process");
    assert_eq!(first_count, 1);
    assert_eq!(second_count, 2, "the Rust handler kept its static state");
}

#[tokio::test]
async fn a_crash_replaces_the_warm_rust_process() {
    if !WarmRust::cargo_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = WarmRust::source(&root, WarmRust::COUNTER);
    let supervisor = WarmRust::supervisor();
    let (before, _) = WarmRust::answer(&supervisor, &source, "rust_before_crash").await;
    let crashed = supervisor
        .invoke(
            &source,
            &WarmRust::request("rust_crash", "crash"),
            &HandlerCancellation::default(),
        )
        .await
        .expect_err("crash");
    assert!(crashed.to_string().contains("TY2104"), "{crashed}");
    let (after, count) = WarmRust::answer(&supervisor, &source, "rust_after_crash").await;
    assert_ne!(before, after, "a new Rust process replaced the crashed one");
    assert_eq!(count, 1, "the new Rust process started with fresh state");
}

#[tokio::test]
async fn a_warm_rust_stream_ends_with_its_end_frame_and_stays_warm() {
    if !WarmRust::cargo_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = WarmRust::source(&root, WarmRust::STREAM);
    let supervisor = WarmRust::supervisor();
    let mut pids = Vec::new();
    for id in ["rust_stream_first", "rust_stream_second"] {
        let request = WarmRust::request(id, "");
        let (result, events) = WarmRust::stream(&supervisor, &source, &request).await;
        result.expect("stream finished");
        assert_eq!(events.len(), 3, "{id}");
        pids.push(WarmRust::first_pid(&events));
    }
    assert_eq!(pids[0], pids[1]);
}

#[tokio::test]
async fn a_failed_warm_rust_stream_replaces_the_process() {
    if !WarmRust::cargo_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = WarmRust::source(&root, WarmRust::STREAM);
    let supervisor = WarmRust::supervisor();
    let failing = WarmRust::request("rust_stream_fails", "fail");
    let (failed, events) = WarmRust::stream(&supervisor, &source, &failing).await;
    assert!(failed.is_err(), "the failed stream reported its error");
    assert_eq!(events.len(), 1);
    let failed_pid = WarmRust::first_pid(&events);
    let later = WarmRust::request("rust_stream_after_failure", "");
    let (result, events) = WarmRust::stream(&supervisor, &source, &later).await;
    result.expect("stream finished");
    assert_eq!(events.len(), 3);
    assert_ne!(failed_pid, WarmRust::first_pid(&events));
}
