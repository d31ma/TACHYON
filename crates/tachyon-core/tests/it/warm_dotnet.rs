#![allow(clippy::expect_used)]

use crate::support::program_probe::ProgramProbe;
use crate::support::toolchain::Toolchain;
use std::fs;
use std::time::Duration;
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HandlerResponse, HttpMethod,
};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
    WarmSettings, YonIsolationPolicy,
};

struct DotnetHandlers;

impl DotnetHandlers {
    const COUNTERS: [(&'static str, &'static str); 2] = [
        (
            "cs",
            r#"
sealed class Handler
{
    static int count;

    public static object POST(HttpRequestMessage request)
    {
        var mode = request.Content is null ? "" : request.Content.ReadAsStringAsync().GetAwaiter().GetResult();
        if (mode == "crash") Environment.Exit(23);
        count += 1;
        var token = Environment.GetEnvironmentVariable("YON_RESOURCE_TOKEN") is not null;
        return new { pid = Environment.ProcessId, count, token };
    }
}
"#,
        ),
        (
            "fs",
            r#"
open System.Net.Http

type Handler() =
    static member val Count = 0 with get, set

    static member POST(request: HttpRequestMessage) : obj =
        let mode = if isNull request.Content then "" else request.Content.ReadAsStringAsync().Result
        if mode = "crash" then System.Environment.Exit 23
        Handler.Count <- Handler.Count + 1
        let token = not (isNull (System.Environment.GetEnvironmentVariable "YON_RESOURCE_TOKEN"))
        box {| pid = System.Environment.ProcessId; count = Handler.Count; token = token |}
"#,
        ),
    ];

    const STREAMS: [(&'static str, &'static str); 2] = [
        (
            "cs",
            r"
sealed class Handler
{
    public static IEnumerable<object> POST()
    {
        for (var index = 0; index < 3; index++) yield return new { pid = Environment.ProcessId, index };
    }
}
",
        ),
        (
            "fs",
            r"
type Handler() =
    static member POST() : seq<obj> =
        seq {
            let pid = System.Environment.ProcessId
            yield box {| pid = pid; index = 0 |}
            yield box {| pid = pid; index = 1 |}
            yield box {| pid = pid; index = 2 |}
        }
",
        ),
    ];
}

fn dotnet_available() -> bool {
    let found = ProgramProbe::answers("dotnet", "--version");
    if !found {
        eprintln!("skipping .NET warm handler execution: dotnet is unavailable");
    }
    Toolchain::checked("dotnet", found)
}

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

async fn streamed_pid(supervisor: &HandlerSupervisor, source: &HandlerSource, id: &str) -> u64 {
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
async fn a_warm_dotnet_handler_serves_later_requests_from_the_same_process() {
    if !dotnet_available() {
        return;
    }
    for (extension, contents) in DotnetHandlers::COUNTERS {
        let root = tempfile::tempdir().expect("project");
        let source = source(&root, extension, contents);
        let supervisor = warm();
        let (first_pid, first_count) = answer(&supervisor, &source, "warm_first").await;
        let (second_pid, second_count) = answer(&supervisor, &source, "warm_second").await;
        assert_eq!(first_pid, second_pid, "{extension} reused its process");
        assert_eq!(first_count, 1, "{extension}");
        assert_eq!(second_count, 2, "{extension} kept its class state");
    }
}

#[tokio::test]
async fn dotnet_process_mode_starts_a_new_process_for_every_request() {
    if !dotnet_available() {
        return;
    }
    for (extension, contents) in DotnetHandlers::COUNTERS {
        let root = tempfile::tempdir().expect("project");
        let source = source(&root, extension, contents);
        let supervisor = supervisor(YonIsolationPolicy::Process);
        let (first_pid, first_count) = answer(&supervisor, &source, "process_first").await;
        let (second_pid, second_count) = answer(&supervisor, &source, "process_second").await;
        assert_ne!(first_pid, second_pid, "{extension}");
        assert_eq!([first_count, second_count], [1, 1], "{extension}");
    }
}

#[tokio::test]
async fn a_dotnet_crash_replaces_the_warm_process() {
    if !dotnet_available() {
        return;
    }
    for (extension, contents) in DotnetHandlers::COUNTERS {
        let root = tempfile::tempdir().expect("project");
        let source = source(&root, extension, contents);
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
        assert!(crashed.to_string().contains("TY2104"), "{extension}: {crashed}");
        let (after, count) = answer(&supervisor, &source, "after_crash").await;
        assert_ne!(before, after, "{extension} started a new process");
        assert_eq!(count, 1, "{extension} started with fresh state");
    }
}

#[tokio::test]
async fn a_warm_dotnet_stream_ends_with_its_end_frame_and_stays_warm() {
    if !dotnet_available() {
        return;
    }
    for (extension, contents) in DotnetHandlers::STREAMS {
        let root = tempfile::tempdir().expect("project");
        let source = source(&root, extension, contents);
        let supervisor = warm();
        let first = streamed_pid(&supervisor, &source, "stream_first").await;
        let second = streamed_pid(&supervisor, &source, "stream_second").await;
        assert_eq!(first, second, "{extension} stream kept its process");
    }
}
