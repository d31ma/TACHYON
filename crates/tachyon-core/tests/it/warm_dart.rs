#![allow(clippy::expect_used)]

use std::fs;
use std::time::Duration;
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HandlerResponse, HttpMethod,
};
use tachyon_core::{
    ApplicationConfig, HandlerCancellation, HandlerSource, HandlerSupervisor,
    HandlerSupervisorOptions, WarmSettings, YonIsolationPolicy,
};

const DART_COUNTER: &str = r"import 'dart:io';

class Handler {
  static int count = 0;

  Object? POST(__YonFrame request) {
    if (request.body() == 'crash') exit(23);
    count += 1;
    return {'pid': pid, 'count': count};
  }
}
";

const DART_STREAM: &str = r"import 'dart:io';

class Handler {
  Stream<Object?> POST() async* {
    for (var index = 0; index < 3; index++) {
      yield {'pid': pid, 'index': index};
    }
  }
}
";

const DART_RESOURCES: &str = r"import 'dart:io';

class Handler {
  Future<Object?> POST() async =>
      {'pid': pid, 'rows': await DataSource().query('SELECT 42 AS answer', [])};
}
";

const SQLITE_APPLICATION: &str = r#"{"datasource":{"driver":"sqlite","url":"sqlite::memory:"}}"#;

fn dart_available() -> bool {
    std::process::Command::new("dart")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn write(root: &tempfile::TempDir, relative: &str, contents: &str) {
    let path = root.path().join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, contents).expect("file");
}

fn source(root: &tempfile::TempDir, contents: &str) -> HandlerSource {
    let relative = "server/workers/warm.dart";
    write(root, relative, contents);
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

fn warm(application: ApplicationConfig) -> HandlerSupervisor {
    let settings = WarmSettings::new(1_000, Duration::from_mins(1));
    HandlerSupervisor::new(HandlerSupervisorOptions {
        application,
        isolation: YonIsolationPolicy::Warm(settings),
        default_timeout: Duration::from_mins(1),
        ..HandlerSupervisorOptions::default()
    })
    .expect("warm supervisor")
}

fn body_json(response: &HandlerResponse) -> serde_json::Value {
    assert!(response.error.is_none(), "{:?}", response.error);
    let body = response.body.as_ref().expect("successful body");
    serde_json::from_str(&body.data).expect("JSON handler result")
}

async fn invoke(
    supervisor: &HandlerSupervisor,
    source: &HandlerSource,
    id: &str,
) -> HandlerResponse {
    supervisor
        .invoke(source, &request(id, ""), &HandlerCancellation::default())
        .await
        .expect("warm response")
}

async fn answer(supervisor: &HandlerSupervisor, source: &HandlerSource, id: &str) -> (u64, u64) {
    let value = body_json(&invoke(supervisor, source, id).await);
    let pid = value["pid"].as_u64().expect("pid");
    let count = value["count"].as_u64().expect("count");
    (pid, count)
}

#[tokio::test]
async fn a_warm_dart_handler_serves_later_requests_from_the_same_process() {
    if !dart_available() {
        eprintln!("skipping: dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, DART_COUNTER);
    let supervisor = warm(ApplicationConfig::default());
    let (first_pid, first_count) = answer(&supervisor, &source, "dart_first").await;
    let (second_pid, second_count) = answer(&supervisor, &source, "dart_second").await;
    assert_eq!(first_pid, second_pid, "the second request reused the process");
    assert_eq!(first_count, 1);
    assert_eq!(second_count, 2, "the class state survived between requests");
}

#[tokio::test]
async fn a_dart_crash_replaces_the_warm_process() {
    if !dart_available() {
        eprintln!("skipping: dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, DART_COUNTER);
    let supervisor = warm(ApplicationConfig::default());
    let (before, _) = answer(&supervisor, &source, "dart_before_crash").await;
    let crashed = supervisor
        .invoke(
            &source,
            &request("dart_crash", "crash"),
            &HandlerCancellation::default(),
        )
        .await
        .expect_err("crash");
    assert!(crashed.to_string().contains("TY2104"), "{crashed}");
    let (after, count) = answer(&supervisor, &source, "dart_after_crash").await;
    assert_ne!(before, after, "a new process answered after the crash");
    assert_eq!(count, 1, "the new process started with fresh state");
}

#[tokio::test]
async fn a_warm_dart_stream_ends_with_its_end_frame_and_stays_warm() {
    if !dart_available() {
        eprintln!("skipping: dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, DART_STREAM);
    let supervisor = warm(ApplicationConfig::default());
    let mut pids = Vec::new();
    for id in ["dart_stream_first", "dart_stream_second"] {
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

#[tokio::test]
async fn a_warm_dart_handler_uses_the_resources_of_each_request() {
    if !dart_available() {
        eprintln!("skipping: dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    write(&root, "server/application.json", SQLITE_APPLICATION);
    let source = source(&root, DART_RESOURCES);
    let application = ApplicationConfig::load(root.path()).expect("configuration");
    let supervisor = warm(application);
    let mut pids = Vec::new();
    for id in ["dart_resources_first", "dart_resources_second"] {
        let value = body_json(&invoke(&supervisor, &source, id).await);
        assert_eq!(value["rows"], serde_json::json!([{"answer": 42}]), "{id}");
        pids.push(value["pid"].as_u64().expect("pid"));
    }
    assert_eq!(pids[0], pids[1], "both requests reached one warm process");
}
