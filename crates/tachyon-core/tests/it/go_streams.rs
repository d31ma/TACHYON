#![allow(clippy::expect_used)]

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    DevServer, DevServerOptions, HandlerCancellation, HandlerSource, HandlerSupervisor,
    HandlerSupervisorOptions,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

struct GoStreams;

impl GoStreams {
    const PID_FILE: &str = "follow.pid";
    const FILES: [(&str, &str); 10] = [
        ("client/pages/tac.html", "<test-ticks />"),
        (
            "client/components/test/ticks/tac.html",
            "<main aria-label=\"Ticks\"><h1>Ticks</h1></main>",
        ),
        (
            "server/models/Tick.go",
            "type Tick struct {\n\tSku   string `json:\"sku\"`\n\tLevel int    `json:\"level\"`\n}\n",
        ),
        (
            "server/models/LogLine.go",
            "type LogLine struct {\n\tNumber int `json:\"number\"`\n}\n",
        ),
        (
            "server/services/TicketService.go",
            "// @Service\ntype TicketService struct{}\n\nfunc (service *TicketService) Levels() []int {\n\treturn []int{1, 2, 3}\n}\n",
        ),
        (
            "server/controllers/TicksController.go",
            "import \"iter\"\n\n// @RequestRoute(\"/ticks\")\n// @Public\ntype TicksController struct {\n\t// @Needs\n\ttickets *TicketService\n}\n\n// @StreamRoute\nfunc (controller *TicksController) StreamTicks() iter.Seq[Tick] {\n\treturn controller.ticksFor(controller.tickets.Levels())\n}\n\nfunc (controller *TicksController) ticksFor(levels []int) iter.Seq[Tick] {\n\treturn func(yield func(Tick) bool) {\n\t\tfor _, level := range levels {\n\t\t\tif !yield(Tick{Sku: \"anvil\", Level: level}) {\n\t\t\t\treturn\n\t\t\t}\n\t\t}\n\t}\n}\n",
        ),
        (
            "server/controllers/ReadingsController.go",
            "import (\n\t\"errors\"\n\t\"iter\"\n)\n\n// @RequestRoute(\"/readings\")\n// @Public\ntype ReadingsController struct{}\n\n// @StreamRoute\nfunc (controller *ReadingsController) StreamReadings() iter.Seq2[Tick, error] {\n\treturn controller.readings()\n}\n\nfunc (controller *ReadingsController) readings() iter.Seq2[Tick, error] {\n\treturn func(yield func(Tick, error) bool) {\n\t\tif yield(Tick{Sku: \"gauge\", Level: 1}, nil) {\n\t\t\tyield(Tick{}, errors.New(\"secret-canary\"))\n\t\t}\n\t}\n}\n",
        ),
        (
            "server/controllers/LogController.go",
            "import (\n\t\"context\"\n\t\"iter\"\n\t\"net/http\"\n\t\"os\"\n\t\"strconv\"\n\t\"time\"\n)\n\n// @RequestRoute(\"/log\")\n// @Public\ntype LogController struct{}\n\n// @StreamRoute\nfunc (controller *LogController) StreamLines(request *http.Request) iter.Seq[LogLine] {\n\tos.WriteFile(request.URL.Query().Get(\"pid\"), []byte(strconv.Itoa(os.Getpid())), 0o600)\n\treturn controller.follow(request.Context())\n}\n\nfunc (controller *LogController) follow(requestContext context.Context) iter.Seq[LogLine] {\n\treturn func(yield func(LogLine) bool) {\n\t\tfor number := 1; requestContext.Err() == nil; number++ {\n\t\t\tif !yield(LogLine{Number: number}) {\n\t\t\t\treturn\n\t\t\t}\n\t\t\ttime.Sleep(20 * time.Millisecond)\n\t\t}\n\t}\n}\n",
        ),
        (
            "server/tests/TicksTestSetup.go",
            "// @TestSetup\ntype TicksTestSetup struct{}\n\nfunc (TicksTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {\n\treturn NewBehaviorConfiguration(), nil\n}\n",
        ),
        (
            "server/tests/TicksControllerTest.go",
            "import \"slices\"\n\n// @Test(TicksTestSetup, {\"partition\":\"tests\"})\ntype TicksControllerTest struct {\n\tbehaviors *BehaviorContext\n}\n\nfunc (test *TicksControllerTest) TestStreamTicksYieldsEveryLevel() {\n\tcontroller := &TicksController{tickets: &TicketService{}}\n\texpected := []Tick{{Sku: \"anvil\", Level: 1}, {Sku: \"anvil\", Level: 2}, {Sku: \"anvil\", Level: 3}}\n\tAssert.Equals(expected, slices.Collect(controller.StreamTicks()))\n}\n",
        ),
    ];

    fn go_available() -> bool {
        Command::new("go")
            .arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, text) in Self::FILES {
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
            std::fs::write(path, text).expect("source");
        }
        root
    }

    async fn serve(root: &Path) -> (u16, tokio::sync::oneshot::Sender<()>) {
        let server = DevServer::bind(
            root,
            &DevServerOptions {
                port: 0,
                watch: false,
                ..DevServerOptions::default()
            },
        )
        .await
        .unwrap_or_else(|failure| panic!("{failure}"));
        let port = server.address().port();
        let (stop, shutdown) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let _served = server
                .run_until(async {
                    let _stopped = shutdown.await;
                })
                .await;
        });
        (port, stop)
    }

    async fn get(port: u16, path: &str) -> String {
        let mut stream = Self::open(port, path, "close").await;
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_mins(1), stream.read_to_end(&mut response))
            .await
            .expect("stream ends within the deadline")
            .expect("read");
        String::from_utf8_lossy(&response).into_owned()
    }

    async fn open(port: u16, path: &str, connection: &str) -> TcpStream {
        let mut stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: {connection}\r\n\r\n");
        stream.write_all(request.as_bytes()).await.expect("request");
        stream
    }

    #[cfg(unix)]
    async fn read_until(stream: &mut TcpStream, needle: &str) -> String {
        let mut received = Vec::new();
        tokio::time::timeout(Duration::from_mins(1), async {
            let mut chunk = [0_u8; 4_096];
            while !String::from_utf8_lossy(&received).contains(needle) {
                let count = stream.read(&mut chunk).await.expect("stream read");
                assert_ne!(count, 0, "stream closed before {needle}");
                received.extend_from_slice(&chunk[..count]);
            }
        })
        .await
        .expect("event within the deadline");
        String::from_utf8_lossy(&received).into_owned()
    }

    async fn build_by_reading_one_line(
        supervisor: &HandlerSupervisor,
        source: &HandlerSource,
        request: &HandlerRequest,
    ) {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let reader = async move {
            receiver.recv().await.expect("a first line");
        };
        let (followed, ()) =
            tokio::join!(supervisor.invoke_streaming(source, request, sender), reader);
        followed.unwrap_or_else(|failure| panic!("the follow stops for a gone reader: {failure}"));
    }

    #[cfg(unix)]
    fn process_exists(pid: &str) -> bool {
        Command::new("kill")
            .args(["-0", pid.trim()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    #[cfg(unix)]
    async fn wait_for_exit(pid: &str) -> bool {
        for _ in 0..100 {
            if !Self::process_exists(pid) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        false
    }
}

#[tokio::test]
async fn a_go_stream_sends_each_typed_item_and_then_the_end_event() {
    if !GoStreams::go_available() {
        eprintln!("skipping: go is unavailable");
        return;
    }
    let root = GoStreams::project();
    let (port, stop) = GoStreams::serve(root.path()).await;
    let streamed = GoStreams::get(port, "/ticks").await;
    assert!(streamed.contains("text/event-stream"), "{streamed}");
    let events: Vec<&str> = streamed
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .collect();
    assert_eq!(
        events,
        [
            "{\"sku\":\"anvil\",\"level\":1}",
            "{\"sku\":\"anvil\",\"level\":2}",
            "{\"sku\":\"anvil\",\"level\":3}",
            "{}"
        ],
        "{streamed}"
    );
    assert!(streamed.contains("event: end\ndata: {}\n\n"), "{streamed}");
    assert!(!streamed.contains("event: error"), "{streamed}");
    let _stopped = stop.send(());
}

#[tokio::test]
async fn a_go_stream_error_ends_the_stream_with_an_error_event() {
    if !GoStreams::go_available() {
        eprintln!("skipping: go is unavailable");
        return;
    }
    let root = GoStreams::project();
    let (port, stop) = GoStreams::serve(root.path()).await;
    let streamed = GoStreams::get(port, "/readings").await;
    assert!(
        streamed.contains("data: {\"sku\":\"gauge\",\"level\":1}"),
        "{streamed}"
    );
    assert!(streamed.contains("event: error"), "{streamed}");
    assert!(!streamed.contains("event: end"), "{streamed}");
    assert!(!streamed.contains("secret-canary"), "{streamed}");
    let _stopped = stop.send(());
}

#[cfg(unix)]
#[tokio::test]
async fn a_go_log_follow_stops_when_the_client_goes_away() {
    if !GoStreams::go_available() {
        eprintln!("skipping: go is unavailable");
        return;
    }
    let root = GoStreams::project();
    let pid_path = root.path().join(GoStreams::PID_FILE);
    let (port, stop) = GoStreams::serve(root.path()).await;
    let path = format!("/log?pid={}", pid_path.display());
    let mut follow = GoStreams::open(port, &path, "keep-alive").await;
    let first = GoStreams::read_until(&mut follow, "data: {\"number\":2}").await;
    assert!(first.contains("data: {\"number\":1}"), "{first}");
    let pid = std::fs::read_to_string(&pid_path).expect("stream process id");
    assert!(GoStreams::process_exists(&pid), "the follow is running");
    drop(follow);
    assert!(
        GoStreams::wait_for_exit(&pid).await,
        "the Go stream process stops after the client goes away"
    );
    let _stopped = stop.send(());
}

#[tokio::test]
async fn a_go_log_follow_ends_cleanly_at_the_request_time_limit() {
    if !GoStreams::go_available() {
        eprintln!("skipping: go is unavailable");
        return;
    }
    let root = GoStreams::project();
    let source = HandlerSource::discover(root.path(), "server/controllers/LogController.go")
        .unwrap_or_else(|failure| panic!("{failure}"));
    assert!(source.streams_method(HttpMethod::Get));
    let pid_path = root.path().join(GoStreams::PID_FILE);
    let target = format!("/log?pid={}", pid_path.display());
    let mut request = HandlerRequest::route("log", "/log", HttpMethod::Get);
    request.target = Some(target);
    request.arguments = Some(Vec::new());
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    GoStreams::build_by_reading_one_line(&supervisor, &source, &request).await;
    request.deadline_ms = Some(3_000);
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1_024);
    supervisor
        .invoke_streaming(&source, &request, sender)
        .await
        .unwrap_or_else(|failure| panic!("the follow ends before the time limit: {failure}"));
    let mut lines = 0;
    while receiver.recv().await.is_some() {
        lines += 1;
    }
    assert!(lines > 1, "{lines}");
}

#[tokio::test]
async fn a_go_test_unit_collects_a_stream_route() {
    if !GoStreams::go_available() {
        eprintln!("skipping: go is unavailable");
        return;
    }
    let root = GoStreams::project();
    let source = HandlerSource::discover(root.path(), "server/tests/TicksControllerTest.go")
        .unwrap_or_else(|failure| panic!("{failure}"));
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let request = HandlerRequest::route("ticks-test", "/", HttpMethod::Post);
    let response = supervisor
        .invoke(&source, &request, &HandlerCancellation::default())
        .await
        .expect("test unit");
    assert!(response.error.is_none(), "{:?}", response.error);
    let report: serde_json::Value =
        serde_json::from_str(&response.body.expect("report").data).expect("JSON report");
    let cases = report["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 1, "{report}");
    assert_eq!(cases[0]["passed"], true, "{report}");
}
