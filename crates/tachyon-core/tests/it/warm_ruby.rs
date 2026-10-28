#![allow(clippy::expect_used)]

use std::fs;
use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HandlerResponse, HttpMethod,
};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions, WarmSettings,
    YonIsolationPolicy,
};

const RUBY_COUNTER: &str = r#"
class Handler
  def self.POST(request)
    exit!(23) if request.body == "crash"
    @count = (@count || 0) + 1
    { "pid" => Process.pid, "count" => @count, "token" => ENV.key?("YON_RESOURCE_TOKEN") }
  end
end
"#;

const RUBY_STREAM: &str = r#"
class Handler
  def self.POST(request)
    Enumerator.new do |events|
      3.times { |index| events << { "pid" => Process.pid, "index" => index } }
    end
  end
end
"#;

const RUBY_RELAY: &str = r#"
class Handler
  def self.POST(request)
    script = "STDIN.read; print JSON.generate(status: 200, body: :relayed)"
    status, _headers, body = relay(["ruby", "-rjson", "-e", script], request)
    { "pid" => Process.pid, "status" => status, "body" => body.join }
  end
end
"#;

fn ruby_available() -> bool {
    let found = Command::new("ruby")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if !found {
        eprintln!("skipping Ruby warm handler execution: ruby is unavailable");
    }
    crate::support::toolchain::Toolchain::checked("ruby", found)
}

fn source(root: &tempfile::TempDir, contents: &str) -> HandlerSource {
    let relative = "server/workers/warm.rb";
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

fn supervisor(isolation: YonIsolationPolicy) -> HandlerSupervisor {
    HandlerSupervisor::new(HandlerSupervisorOptions {
        isolation,
        default_timeout: Duration::from_secs(10),
        ..HandlerSupervisorOptions::default()
    })
    .expect("supervisor")
}

fn warm() -> HandlerSupervisor {
    let settings = WarmSettings::new(1_000, Duration::from_mins(1));
    supervisor(YonIsolationPolicy::Warm(settings))
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
    assert_eq!(value["token"], false, "{id} saw no resource token");
    let pid = value["pid"].as_u64().expect("pid");
    let count = value["count"].as_u64().expect("count");
    (pid, count)
}

#[tokio::test]
async fn a_warm_ruby_handler_serves_later_requests_from_the_same_process() {
    if !ruby_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, RUBY_COUNTER);
    let supervisor = warm();
    let (first_pid, first_count) = answer(&supervisor, &source, "warm_first").await;
    let (second_pid, second_count) = answer(&supervisor, &source, "warm_second").await;
    assert_eq!(first_pid, second_pid, "the Ruby process was reused");
    assert_eq!(first_count, 1);
    assert_eq!(second_count, 2, "the Ruby handler kept its class state");
}

#[tokio::test]
async fn ruby_process_mode_starts_a_new_process_for_every_request() {
    if !ruby_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, RUBY_COUNTER);
    let supervisor = supervisor(YonIsolationPolicy::Process);
    let (first_pid, first_count) = answer(&supervisor, &source, "process_first").await;
    let (second_pid, second_count) = answer(&supervisor, &source, "process_second").await;
    assert_ne!(first_pid, second_pid);
    assert_eq!([first_count, second_count], [1, 1]);
}

#[tokio::test]
async fn a_ruby_crash_replaces_the_warm_process() {
    if !ruby_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, RUBY_COUNTER);
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
    assert_ne!(before, after, "a new Ruby process started");
    assert_eq!(count, 1, "the new Ruby process started with fresh state");
}

#[tokio::test]
async fn a_warm_ruby_stream_ends_with_its_end_frame_and_stays_warm() {
    if !ruby_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, RUBY_STREAM);
    let supervisor = warm();
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

#[tokio::test]
async fn a_warm_ruby_handler_relays_on_every_request() {
    if !ruby_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, RUBY_RELAY);
    let supervisor = warm();
    let mut pids = Vec::new();
    for id in ["relay_first", "relay_second", "relay_third"] {
        let response = supervisor
            .invoke(&source, &request(id, ""), &HandlerCancellation::default())
            .await
            .unwrap_or_else(|failure| panic!("{id} relayed: {failure}"));
        let value = body_json(&response);
        assert_eq!(value["status"], 200, "{id}: {value}");
        assert_eq!(value["body"], "relayed", "{id}: {value}");
        pids.push(value["pid"].as_u64().expect("pid"));
    }
    assert!(
        pids.iter().all(|pid| *pid == pids[0]),
        "one warm process relayed every time: {pids:?}"
    );
}
