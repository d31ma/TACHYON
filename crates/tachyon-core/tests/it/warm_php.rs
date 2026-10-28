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

const PHP_COUNTER: &str = r"<?php
final class Handler
{
    public static int $count = 0;

    public static function POST(): ?array
    {
        $mode = file_get_contents('php://input');
        if ($mode === 'crash') {
            exit(23);
        }
        self::$count += 1;
        $probe = $_SERVER['HTTP_X_PROBE'] ?? null;
        $answer = ['pid' => getmypid(), 'count' => self::$count, 'probe' => $probe];
        if ($mode === 'header') {
            header('X-Warm: yes');
            http_response_code(201);
            echo json_encode($answer);
            return null;
        }
        return $answer;
    }
}
";

const PHP_STREAM: &str = r"<?php
final class Handler
{
    public static function POST(): Generator
    {
        for ($index = 0; $index < 3; $index++) {
            yield ['pid' => getmypid(), 'index' => $index];
        }
    }
}
";

fn php_available() -> bool {
    let found = crate::support::program_probe::ProgramProbe::answers("php-cgi", "--version");
    if !found {
        eprintln!("skipping warm PHP handlers: php-cgi unavailable");
    }
    crate::support::toolchain::Toolchain::checked("php-cgi", found)
}

fn source(root: &tempfile::TempDir, contents: &str) -> HandlerSource {
    let relative = "server/workers/warm.php";
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

fn warm() -> HandlerSupervisor {
    let settings = WarmSettings::new(1_000, Duration::from_mins(1));
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

async fn invoke(
    supervisor: &HandlerSupervisor,
    source: &HandlerSource,
    request: &HandlerRequest,
) -> HandlerResponse {
    supervisor
        .invoke(source, request, &HandlerCancellation::default())
        .await
        .expect("PHP response")
}

async fn answer(supervisor: &HandlerSupervisor, source: &HandlerSource, id: &str) -> (u64, u64) {
    let response = invoke(supervisor, source, &request(id, "")).await;
    let value = body_json(&response);
    let pid = value["pid"].as_u64().expect("pid");
    let count = value["count"].as_u64().expect("count");
    (pid, count)
}

#[tokio::test]
async fn a_warm_php_handler_serves_later_requests_from_the_same_process() {
    if !php_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, PHP_COUNTER);
    let supervisor = warm();
    let (first_pid, first_count) = answer(&supervisor, &source, "php_warm_first").await;
    let (second_pid, second_count) = answer(&supervisor, &source, "php_warm_second").await;
    assert_eq!(first_pid, second_pid, "PHP reused its process");
    assert_eq!(first_count, 1);
    assert_eq!(second_count, 2, "PHP kept its class state");
}

#[tokio::test]
async fn a_warm_php_request_does_not_see_the_previous_request_headers_or_status() {
    if !php_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, PHP_COUNTER);
    let supervisor = warm();
    let mut first = request("php_headers_first", "header");
    first
        .headers
        .insert(String::from("x-probe"), vec![String::from("one")]);
    let written = invoke(&supervisor, &source, &first).await;
    assert_eq!(written.status, 201, "{written:?}");
    assert_eq!(
        written.headers.get("x-warm").map(Vec::as_slice),
        Some(&[String::from("yes")][..]),
        "{written:?}"
    );
    assert_eq!(body_json(&written)["probe"], "one");
    let plain = invoke(&supervisor, &source, &request("php_headers_second", "")).await;
    assert_eq!(plain.status, 200, "{plain:?}");
    assert!(!plain.headers.contains_key("x-warm"), "{plain:?}");
    let value = body_json(&plain);
    assert_eq!(value["probe"], serde_json::Value::Null, "{value}");
    assert_eq!(value["pid"], body_json(&written)["pid"], "{value}");
    assert_eq!(value["count"], 2, "{value}");
}

#[tokio::test]
async fn a_php_crash_replaces_the_warm_process() {
    if !php_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, PHP_COUNTER);
    let supervisor = warm();
    let (before, _) = answer(&supervisor, &source, "php_before_crash").await;
    let crashed = supervisor
        .invoke(
            &source,
            &request("php_crash", "crash"),
            &HandlerCancellation::default(),
        )
        .await
        .expect_err("crash");
    assert!(crashed.to_string().contains("TY2104"), "{crashed}");
    let (after, count) = answer(&supervisor, &source, "php_after_crash").await;
    assert_ne!(before, after, "PHP started a new process");
    assert_eq!(count, 1, "PHP started with fresh state");
}

#[tokio::test]
async fn php_process_mode_still_starts_a_new_process_for_every_request() {
    if !php_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, PHP_COUNTER);
    let supervisor = HandlerSupervisor::new(HandlerSupervisorOptions {
        isolation: YonIsolationPolicy::Process,
        ..HandlerSupervisorOptions::default()
    })
    .expect("process supervisor");
    let (first_pid, first_count) = answer(&supervisor, &source, "php_process_first").await;
    let (second_pid, second_count) = answer(&supervisor, &source, "php_process_second").await;
    assert_ne!(first_pid, second_pid);
    assert_eq!([first_count, second_count], [1, 1]);
}

#[tokio::test]
async fn a_warm_php_stream_ends_with_its_end_frame_and_stays_warm() {
    if !php_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, PHP_STREAM);
    let supervisor = warm();
    let mut pids = Vec::new();
    for id in ["php_stream_first", "php_stream_second"] {
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
