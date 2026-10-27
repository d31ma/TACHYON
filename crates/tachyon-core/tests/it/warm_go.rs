#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HandlerResponse, HttpMethod,
};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
    WarmSettings, YonIsolationPolicy,
};

struct WarmGo;

impl WarmGo {
    const COUNTER_PATH: &str = "server/workers/warm.go";
    const COUNTER: &str = r#"import (
	"io"
	"net/http"
	"os"
	"strconv"
)

type Handler struct{}

func (Handler) POST(request *http.Request) any {
	mode, _ := io.ReadAll(request.Body)
	if string(mode) == "crash" {
		os.Exit(23)
	}
	count, _ := strconv.Atoi(os.Getenv("WARM_GO_COUNT"))
	count++
	os.Setenv("WARM_GO_COUNT", strconv.Itoa(count))
	return map[string]int{"pid": os.Getpid(), "count": count}
}
"#;
    const STREAM_PATH: &str = "server/controllers/BeatsController.go";
    const STREAM_FILES: [(&str, &str); 2] = [
        (
            "server/models/Beat.go",
            "type Beat struct {\n\tPid   int `json:\"pid\"`\n\tIndex int `json:\"index\"`\n}\n",
        ),
        (
            "server/controllers/BeatsController.go",
            "import (\n\t\"iter\"\n\t\"os\"\n)\n\n// @RequestRoute(\"/beats\")\n// @Public\ntype BeatsController struct{}\n\n// @StreamRoute\nfunc (controller *BeatsController) StreamBeats() iter.Seq[Beat] {\n\treturn controller.beats()\n}\n\nfunc (controller *BeatsController) beats() iter.Seq[Beat] {\n\treturn func(yield func(Beat) bool) {\n\t\tfor index := range 3 {\n\t\t\tif !yield(Beat{Pid: os.Getpid(), Index: index}) {\n\t\t\t\treturn\n\t\t\t}\n\t\t}\n\t}\n}\n",
        ),
    ];

    fn go_usable() -> bool {
        let found = Command::new("go")
            .arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        let required = cfg!(unix) && std::env::var_os("CI").is_some();
        assert!(
            found || !required,
            "go is required for the Go warm handler tests in CI"
        );
        if !found {
            eprintln!("skipping Go warm handler execution: go is unavailable");
        }
        found
    }

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("handler parent")).expect("handler directory");
        fs::write(path, contents).expect("handler source");
    }

    fn counter(root: &tempfile::TempDir) -> HandlerSource {
        Self::write(root.path(), Self::COUNTER_PATH, Self::COUNTER);
        HandlerSource::discover(root.path(), Self::COUNTER_PATH).expect("validated source")
    }

    fn stream(root: &tempfile::TempDir) -> HandlerSource {
        for (relative, contents) in Self::STREAM_FILES {
            Self::write(root.path(), relative, contents);
        }
        HandlerSource::discover(root.path(), Self::STREAM_PATH).expect("validated source")
    }

    fn request(id: &str, mode: &str) -> HandlerRequest {
        let mut request = HandlerRequest::route(id, "/warm", HttpMethod::Post);
        request.body = Some(HandlerBody {
            encoding: HandlerBodyEncoding::Utf8,
            data: String::from(mode),
        });
        request
    }

    fn stream_request(id: &str) -> HandlerRequest {
        let mut request = HandlerRequest::route(id, "/beats", HttpMethod::Get);
        request.arguments = Some(Vec::new());
        request
    }

    fn supervisor(isolation: YonIsolationPolicy) -> HandlerSupervisor {
        HandlerSupervisor::new(HandlerSupervisorOptions {
            isolation,
            default_timeout: Duration::from_secs(30),
            ..HandlerSupervisorOptions::default()
        })
        .expect("supervisor")
    }

    fn warm() -> HandlerSupervisor {
        let settings = WarmSettings::new(1_000, Duration::from_secs(60));
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
    ) -> (u64, u64) {
        let response = supervisor
            .invoke(source, &Self::request(id, ""), &HandlerCancellation::default())
            .await
            .expect("Go response");
        let value = Self::body_json(&response);
        let pid = value["pid"].as_u64().expect("pid");
        let count = value["count"].as_u64().expect("count");
        (pid, count)
    }
}

#[tokio::test]
async fn a_warm_go_handler_serves_later_requests_from_the_same_process() {
    if !WarmGo::go_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = WarmGo::counter(&root);
    let supervisor = WarmGo::warm();
    let (first_pid, first_count) = WarmGo::answer(&supervisor, &source, "go_first").await;
    let (second_pid, second_count) = WarmGo::answer(&supervisor, &source, "go_second").await;
    assert_eq!(first_pid, second_pid, "the Go process was reused");
    assert_eq!(first_count, 1);
    assert_eq!(second_count, 2, "the Go process kept its state");
}

#[tokio::test]
async fn a_go_handler_in_process_mode_starts_a_new_process_for_every_request() {
    if !WarmGo::go_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = WarmGo::counter(&root);
    let supervisor = WarmGo::supervisor(YonIsolationPolicy::Process);
    let (first_pid, first_count) = WarmGo::answer(&supervisor, &source, "go_process_1").await;
    let (second_pid, second_count) = WarmGo::answer(&supervisor, &source, "go_process_2").await;
    assert_ne!(first_pid, second_pid);
    assert_eq!([first_count, second_count], [1, 1]);
}

#[tokio::test]
async fn a_go_crash_replaces_the_warm_process() {
    if !WarmGo::go_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = WarmGo::counter(&root);
    let supervisor = WarmGo::warm();
    let (before, _) = WarmGo::answer(&supervisor, &source, "go_before_crash").await;
    let crashed = supervisor
        .invoke(
            &source,
            &WarmGo::request("go_crash", "crash"),
            &HandlerCancellation::default(),
        )
        .await
        .expect_err("crash");
    assert!(crashed.to_string().contains("TY2104"), "{crashed}");
    let (after, count) = WarmGo::answer(&supervisor, &source, "go_after_crash").await;
    assert_ne!(before, after, "a new Go process started");
    assert_eq!(count, 1, "the new Go process started with fresh state");
}

#[tokio::test]
async fn a_warm_go_stream_ends_with_its_end_frame_and_stays_warm() {
    if !WarmGo::go_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = WarmGo::stream(&root);
    assert!(source.streams_method(HttpMethod::Get));
    let supervisor = WarmGo::warm();
    let mut pids = Vec::new();
    for id in ["go_stream_first", "go_stream_second"] {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
        let request = WarmGo::stream_request(id);
        let invocation = supervisor.invoke_streaming(&source, &request, sender);
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
        pids.push(first["pid"].as_u64().expect("pid"));
    }
    assert_eq!(pids[0], pids[1], "the Go stream process stayed warm");
}
