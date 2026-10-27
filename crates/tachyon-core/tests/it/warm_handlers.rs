#![allow(clippy::expect_used)]

use std::fs;
use std::time::Duration;
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HandlerResponse, HttpMethod,
};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
    WarmSettings, YonIsolationPolicy,
};

const JAVASCRIPT_COUNTER: &str = r"
export class Handler {
  static count = 0

  static async POST(request) {
    const mode = await request.text()
    if (mode === 'crash') process.exit(23)
    if (mode === 'sleep') await new Promise((resolve) => setTimeout(resolve, 5000))
    if (mode === 'stray') setTimeout(() => process.stdout.write('stray bytes'), 20)
    Handler.count += 1
    return { pid: process.pid, count: Handler.count }
  }
}
";

const PYTHON_COUNTER: &str = r#"
import os
import sys


class Handler:
    count = 0

    @staticmethod
    def POST(request):
        mode = request.data.decode()
        if mode == "crash":
            sys.stderr.flush()
            os._exit(23)
        Handler.count += 1
        return {"pid": os.getpid(), "count": Handler.count}
"#;

const PYTHON_STREAM: &str = r#"
import os


class Handler:
    @staticmethod
    def POST(request):
        for index in range(3):
            yield {"pid": os.getpid(), "index": index}
"#;

fn source(root: &tempfile::TempDir, extension: &str, contents: &str) -> HandlerSource {
    let relative = format!("server/workers/warm.{extension}");
    let path = root.path().join(&relative);
    fs::create_dir_all(path.parent().expect("handler parent")).expect("handler directory");
    fs::write(path, contents).expect("handler source");
    HandlerSource::discover(root.path(), &relative).expect("validated source")
}

fn request(id: &str, mode: &str) -> HandlerRequest {
    let mut request = HandlerRequest::route(id, "/warm", HttpMethod::Post);
    request.body = Some(HandlerBody {
        encoding: HandlerBodyEncoding::Utf8,
        data: String::from(mode),
    });
    request
}

fn warm(max_requests: u32, idle_limit: Duration) -> HandlerSupervisor {
    let settings = WarmSettings::new(max_requests, idle_limit);
    HandlerSupervisor::new(HandlerSupervisorOptions {
        isolation: YonIsolationPolicy::Warm(settings),
        default_timeout: Duration::from_secs(10),
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

#[test]
fn warm_is_the_default_isolation() {
    assert!(matches!(
        HandlerSupervisorOptions::default().isolation,
        YonIsolationPolicy::Warm(_)
    ));
}

#[tokio::test]
async fn a_warm_handler_serves_later_requests_from_the_same_process() {
    for (extension, contents) in [("js", JAVASCRIPT_COUNTER), ("py", PYTHON_COUNTER)] {
        let root = tempfile::tempdir().expect("project");
        let source = source(&root, extension, contents);
        let supervisor = warm(1_000, Duration::from_mins(1));
        let (first_pid, first_count) = answer(&supervisor, &source, "warm_first").await;
        let (second_pid, second_count) = answer(&supervisor, &source, "warm_second").await;
        assert_eq!(first_pid, second_pid, "{extension} reused its process");
        assert_eq!(first_count, 1, "{extension}");
        assert_eq!(second_count, 2, "{extension} kept its module state");
    }
}

#[tokio::test]
async fn process_mode_starts_a_new_process_for_every_request() {
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, "js", JAVASCRIPT_COUNTER);
    let supervisor = HandlerSupervisor::new(HandlerSupervisorOptions {
        isolation: YonIsolationPolicy::Process,
        ..HandlerSupervisorOptions::default()
    })
    .expect("process supervisor");
    let (first_pid, first_count) = answer(&supervisor, &source, "process_first").await;
    let (second_pid, second_count) = answer(&supervisor, &source, "process_second").await;
    assert_ne!(first_pid, second_pid);
    assert_eq!([first_count, second_count], [1, 1]);
}

#[tokio::test]
async fn a_crash_replaces_the_warm_process() {
    for (extension, contents) in [("js", JAVASCRIPT_COUNTER), ("py", PYTHON_COUNTER)] {
        let root = tempfile::tempdir().expect("project");
        let source = source(&root, extension, contents);
        let supervisor = warm(1_000, Duration::from_mins(1));
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
        assert_ne!(before, after, "{extension} started a new process");
        assert_eq!(count, 1, "{extension} started with fresh state");
    }
}

#[tokio::test]
async fn a_timeout_replaces_the_warm_process() {
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, "js", JAVASCRIPT_COUNTER);
    let supervisor = HandlerSupervisor::new(HandlerSupervisorOptions {
        isolation: YonIsolationPolicy::Warm(WarmSettings::default()),
        default_timeout: Duration::from_millis(1_500),
        cancellation_grace: Duration::from_millis(20),
        ..HandlerSupervisorOptions::default()
    })
    .expect("warm supervisor");
    let (before, _) = answer(&supervisor, &source, "before_timeout").await;
    let timed_out = supervisor
        .invoke(
            &source,
            &request("timeout", "sleep"),
            &HandlerCancellation::default(),
        )
        .await
        .expect_err("timeout");
    assert!(timed_out.to_string().contains("TY2110"), "{timed_out}");
    let (after, count) = answer(&supervisor, &source, "after_timeout").await;
    assert_ne!(before, after);
    assert_eq!(count, 1);
}

#[tokio::test]
async fn stray_output_after_a_response_replaces_the_warm_process() {
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, "js", JAVASCRIPT_COUNTER);
    let supervisor = warm(1_000, Duration::from_mins(1));
    let response = supervisor
        .invoke(
            &source,
            &request("stray", "stray"),
            &HandlerCancellation::default(),
        )
        .await
        .expect("the request that wrote stray output still answered");
    let stray_pid = body_json(&response)["pid"].as_u64().expect("pid");
    tokio::time::sleep(Duration::from_millis(200)).await;
    let (after, count) = answer(&supervisor, &source, "after_stray").await;
    assert_ne!(stray_pid, after);
    assert_eq!(count, 1);
}

#[tokio::test]
async fn the_request_limit_replaces_the_warm_process() {
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, "py", PYTHON_COUNTER);
    let supervisor = warm(2, Duration::from_mins(1));
    let (first, _) = answer(&supervisor, &source, "limit_first").await;
    let (second, _) = answer(&supervisor, &source, "limit_second").await;
    let (third, count) = answer(&supervisor, &source, "limit_third").await;
    assert_eq!(first, second);
    assert_ne!(second, third);
    assert_eq!(count, 1);
}

#[tokio::test]
async fn an_idle_warm_process_stops_after_its_idle_limit() {
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, "js", JAVASCRIPT_COUNTER);
    let supervisor = warm(1_000, Duration::from_secs(1));
    let (before, _) = answer(&supervisor, &source, "before_idle").await;
    tokio::time::sleep(Duration::from_millis(2_500)).await;
    let (after, count) = answer(&supervisor, &source, "after_idle").await;
    assert_ne!(before, after);
    assert_eq!(count, 1);
}

#[tokio::test]
async fn concurrent_requests_get_separate_warm_processes() {
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, "py", PYTHON_COUNTER);
    let supervisor = warm(1_000, Duration::from_mins(1));
    let (one, two) = tokio::join!(
        answer(&supervisor, &source, "together_one"),
        answer(&supervisor, &source, "together_two")
    );
    assert_ne!(one.0, two.0);
    let (later, _) = answer(&supervisor, &source, "together_later").await;
    assert!([one.0, two.0].contains(&later));
}

#[tokio::test]
async fn a_warm_stream_ends_with_its_end_frame_and_stays_warm() {
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, "py", PYTHON_STREAM);
    let supervisor = warm(1_000, Duration::from_mins(1));
    let mut pids = Vec::new();
    for id in ["stream_first", "stream_second"] {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
        let streamed = request(id, "");
        let invocation = supervisor.invoke_streaming(&source, &streamed, sender);
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
        pids.push(first["pid"].as_u64().expect("pid"));
    }
    assert_eq!(pids[0], pids[1]);
}
