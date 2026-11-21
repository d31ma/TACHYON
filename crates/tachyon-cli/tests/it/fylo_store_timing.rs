#![allow(clippy::expect_used)]

use crate::windows_egress::WindowsEgress;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

struct TimedProject {
    workspace: tempfile::TempDir,
}

impl TimedProject {
    const READY_SECONDS: u64 = 120;
    const CONTROLLER: &'static str = r#"from typing import Annotated

@request_route("/events")
@public
class EventsController:
    data_source: Annotated[DataSource, needs]

    @get_route
    def get_events(self) -> dict[str, object]:
        created = self.data_source.collection_exists("hd-events")
        return {"exists": created}
"#;

    const WRITER: &'static str = r#"from typing import Annotated

@request_route("/writes")
@public
class WritesController:
    data_source: Annotated[DataSource, needs]

    @get_route
    def get_writes(self) -> dict[str, object]:
        saved = self.data_source.insert("hd-events", {"kind": "deploy"})
        return {"saved": saved}
"#;

    fn new(settings: &serde_json::Value) -> Self {
        let project = Self::empty();
        let mut datasource = serde_json::json!({"driver": "fylo", "url": project.root_url()});
        for (key, value) in settings.as_object().expect("datasource settings") {
            datasource[key] = value.clone();
        }
        let application = serde_json::json!({ "datasource": datasource });
        project.write("server/application.json", &application.to_string());
        project
    }

    fn like_mesail_lambda() -> Self {
        let project = Self::empty();
        project.write("server/application.json", "{}");
        project.write(
            "server/application-lambda.json",
            r#"{ "server": { "host": "127.0.0.1" } }"#,
        );
        let environment = format!(
            "YON_DATASOURCE_DRIVER=fylo\nYON_DATASOURCE_URL={}\n",
            project.root_url()
        );
        project.write(".env", &environment);
        project
    }

    fn empty() -> Self {
        let workspace = tempfile::tempdir().expect("workspace");
        std::fs::create_dir_all(workspace.path().join("fylo-root")).expect("FYLO root");
        let project = Self { workspace };
        project.write("server/controllers/EventsController.py", Self::CONTROLLER);
        project.write("server/controllers/WritesController.py", Self::WRITER);
        project.write("client/pages/tac.html", "<test-view1 />");
        project.write(
            "client/components/test/view1/tac.html",
            "<main>Ready</main>",
        );
        project.write("dist/web/index.html", "<main>Ready</main>");
        project
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.project().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, contents).expect("fixture");
    }

    fn project(&self) -> PathBuf {
        self.workspace.path().join("project")
    }

    fn root(&self) -> PathBuf {
        std::fs::canonicalize(self.workspace.path().join("fylo-root")).expect("FYLO root")
    }

    fn root_url(&self) -> String {
        let url = self.root().display().to_string().replace('\\', "/");
        let url = url.strip_prefix("//?/").unwrap_or(&url).to_owned();
        format!("fylo:///{}", url.trim_start_matches('/'))
    }

    fn hold_root(&self) -> Child {
        let mut holder = Command::new("fylo")
            .args(["exec", "--loop", "--root"])
            .arg(self.root())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("second fylo loop");
        let request = r#"{"op":"createCollection","collection":"hd-events"}"#;
        writeln!(holder.stdin.as_mut().expect("input"), "{request}").expect("request");
        let mut answer = String::new();
        BufReader::new(holder.stdout.as_mut().expect("output"))
            .read_line(&mut answer)
            .expect("answer");
        assert!(answer.contains(r#""ok":true"#), "{answer}");
        holder
    }

    fn start_and_get(&self, profile: Option<&str>) -> (String, String) {
        let (mut server, origin) = self.start(profile);
        let answer = Self::get(&origin, "/events");
        let _killed = server.kill();
        (answer, Self::finish(server))
    }

    fn finish(mut server: Child) -> String {
        let _waited = server.wait();
        let mut stderr = String::new();
        server
            .stderr
            .take()
            .expect("stderr")
            .read_to_string(&mut stderr)
            .expect("server stderr");
        stderr
    }

    fn start(&self, profile: Option<&str>) -> (Child, String) {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
        command
            .arg("start")
            .arg(self.project())
            .args(["--host", "127.0.0.1", "--port", "0"])
            .env_remove("YON_PROFILE")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(profile) = profile {
            command.env("YON_PROFILE", profile);
        }
        WindowsEgress::proxied_where_unconfinable(&mut command);
        let mut server = command.spawn().expect("ty start");
        let origin = Self::origin(server.stdout.take().expect("stdout"));
        (server, origin)
    }

    fn origin(stdout: ChildStdout) -> String {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let _read = BufReader::new(stdout).read_line(&mut line);
            let _sent = sender.send(line);
        });
        let line = receiver
            .recv_timeout(Duration::from_secs(Self::READY_SECONDS))
            .expect("bounded readiness");
        let start = line.find("http://").expect("ready line names the origin");
        line[start..].trim().trim_end_matches('/').to_owned()
    }

    fn get(origin: &str, path: &str) -> String {
        let address = origin.trim_start_matches("http://");
        let mut stream = TcpStream::connect(address).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(Self::READY_SECONDS)))
            .expect("read timeout");
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).expect("request");
        let mut answer = String::new();
        stream.read_to_string(&mut answer).expect("answer");
        answer
    }

    fn events(stderr: &str, name: &str) -> Vec<serde_json::Value> {
        stderr
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|event| event["event"] == name)
            .collect()
    }

    fn available(program: &str) -> bool {
        let found = Command::new(program)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success());
        assert!(
            found || std::env::var("TAC_STORAGE_REQUIRE_TOOLCHAINS").is_err(),
            "{program} is required on this runner"
        );
        found
    }

    fn python() -> &'static str {
        if cfg!(windows) { "python" } else { "python3" }
    }

    fn assert_slow_line_after_a_held_root(stderr: &str) {
        let slow = Self::events(stderr, "handler.storage_slow");
        assert_eq!(slow.len(), 1, "{stderr}");
        let line = &slow[0];
        assert_eq!(line["datasource"], "fylo", "{line}");
        assert_eq!(line["operation"], "document.collectionExists", "{line}");
        assert_eq!(line["collection"], serde_json::Value::Null, "{line}");
        assert!(line["startup_ms"].as_u64().is_some(), "{line}");
        assert_eq!(line["result"], "TY2160", "{line}");
        assert!(
            line["request_id"].as_str().is_some_and(|id| !id.is_empty()),
            "{line}"
        );
        let waited = line["wait_ms"].as_u64().expect("wait_ms");
        let called = line["call_ms"].as_u64().expect("call_ms");
        assert!(waited >= 1_500, "{line}");
        assert!(called < waited, "{line}");
    }
}

#[test]
fn ty_start_logs_a_slow_store_call_with_its_wait_time() {
    if !TimedProject::available("fylo") || !TimedProject::available(TimedProject::python()) {
        return;
    }
    let project = TimedProject::new(&serde_json::json!({}));
    let mut holder = project.hold_root();
    let (answer, stderr) = project.start_and_get(None);
    holder.kill().expect("stop second fylo loop");
    holder.wait().expect("reap second fylo loop");
    assert!(!answer.starts_with("HTTP/1.1 200"), "{answer}");
    TimedProject::assert_slow_line_after_a_held_root(&stderr);
}

#[test]
fn the_lambda_setup_of_mesail_logs_the_same_slow_line() {
    if !TimedProject::available("fylo") || !TimedProject::available(TimedProject::python()) {
        return;
    }
    let project = TimedProject::like_mesail_lambda();
    let mut holder = project.hold_root();
    let (answer, stderr) = project.start_and_get(Some("lambda"));
    holder.kill().expect("stop second fylo loop");
    holder.wait().expect("reap second fylo loop");
    assert!(!answer.starts_with("HTTP/1.1 200"), "{answer}");
    TimedProject::assert_slow_line_after_a_held_root(&stderr);
}

#[test]
fn logcalls_writes_a_line_for_every_store_call() {
    if !TimedProject::available("fylo") || !TimedProject::available(TimedProject::python()) {
        return;
    }
    let project = TimedProject::new(&serde_json::json!({"logcalls": true}));
    let (answer, stderr) = project.start_and_get(None);
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}\n{stderr}");
    let calls = TimedProject::events(&stderr, "handler.storage_call");
    assert_eq!(calls.len(), 1, "{stderr}");
    assert_eq!(calls[0]["operation"], "document.collectionExists");
    assert_eq!(calls[0]["result"], "ok");
    assert!(
        TimedProject::events(&stderr, "handler.storage_slow").is_empty(),
        "{stderr}"
    );
}

#[test]
fn a_fast_store_call_writes_no_line_by_default() {
    if !TimedProject::available("fylo") || !TimedProject::available(TimedProject::python()) {
        return;
    }
    let project = TimedProject::new(&serde_json::json!({}));
    let (answer, stderr) = project.start_and_get(None);
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}\n{stderr}");
    assert!(
        TimedProject::events(&stderr, "handler.storage_slow").is_empty(),
        "{stderr}"
    );
    assert!(
        TimedProject::events(&stderr, "handler.storage_call").is_empty(),
        "{stderr}"
    );
}

#[test]
fn a_bad_slowcall_setting_stops_the_store() {
    if !TimedProject::available("fylo") || !TimedProject::available(TimedProject::python()) {
        return;
    }
    let project = TimedProject::new(&serde_json::json!({"slowcall": 0}));
    let output = Command::new(env!("CARGO_BIN_EXE_ty"))
        .arg("start")
        .arg(project.project())
        .args(["--host", "127.0.0.1", "--port", "0"])
        .env_remove("YON_PROFILE")
        .output()
        .expect("ty start");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("slowcall"), "{stderr}");
}

#[cfg(unix)]
struct Signals;

#[cfg(unix)]
impl Signals {
    fn send(signal: &str, process_id: &str) -> bool {
        Command::new("kill")
            .args([signal, process_id])
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn fylo_of(root: &std::path::Path) -> Option<String> {
        let output = Command::new("pgrep")
            .arg("-f")
            .arg(format!("fylo exec --loop --root {}", root.display()))
            .output()
            .expect("pgrep");
        let ids = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        ids.lines().next().map(str::to_owned)
    }

    fn exits_within(server: &mut Child, limit: Duration) -> bool {
        let started = std::time::Instant::now();
        while started.elapsed() < limit {
            if server.try_wait().expect("ty status").is_some() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    fn stored_events(root: &std::path::Path) -> usize {
        let output = Command::new("fylo")
            .args([
                "exec",
                "--request",
                r#"{"op":"storeCount","collection":"hd-events","query":{"where":[]}}"#,
                "--root",
            ])
            .arg(root)
            .output()
            .expect("fylo storeCount");
        let answer: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("fylo answer");
        answer["result"]
            .as_u64()
            .and_then(|count| usize::try_from(count).ok())
            .unwrap_or_default()
    }

    fn create_collection(root: &std::path::Path) {
        let created = Command::new("fylo")
            .args([
                "exec",
                "--request",
                r#"{"op":"createCollection","collection":"hd-events"}"#,
                "--root",
            ])
            .arg(root)
            .output()
            .expect("fylo createCollection");
        assert!(created.status.success(), "{created:?}");
    }

    fn assert_closed_at_shutdown(stderr: &str) {
        let stopped = TimedProject::events(stderr, "storage.fylo_process_stopped");
        assert_eq!(stopped.len(), 1, "{stderr}");
        assert_eq!(stopped[0]["operation"], "shutdown", "{stderr}");
        assert_eq!(stopped[0]["stop"], "closed", "{stderr}");
    }
}

#[cfg(unix)]
#[test]
fn sigterm_closes_an_idle_fylo_and_ty_exits() {
    if !TimedProject::available("fylo") || !TimedProject::available(TimedProject::python()) {
        return;
    }
    let project = TimedProject::new(&serde_json::json!({}));
    let (mut server, origin) = project.start(None);
    let answer = TimedProject::get(&origin, "/events");
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    let fylo = Signals::fylo_of(&project.root()).expect("ty started fylo");
    assert!(Signals::send("-TERM", &server.id().to_string()));
    assert!(
        Signals::exits_within(&mut server, Duration::from_secs(3)),
        "ty exits after SIGTERM"
    );
    let stderr = TimedProject::finish(server);
    Signals::assert_closed_at_shutdown(&stderr);
    assert!(!Signals::send("-0", &fylo), "fylo has exited");
}

#[cfg(unix)]
#[test]
fn sigterm_during_a_write_keeps_the_write_and_ty_exits() {
    if !TimedProject::available("fylo") || !TimedProject::available(TimedProject::python()) {
        return;
    }
    let project = TimedProject::new(&serde_json::json!({}));
    Signals::create_collection(&project.root());
    let (mut server, origin) = project.start(None);
    let answer = TimedProject::get(&origin, "/events");
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    let fylo = Signals::fylo_of(&project.root()).expect("ty started fylo");
    assert!(Signals::send("-STOP", &fylo));
    let writer = std::thread::spawn(move || TimedProject::get(&origin, "/writes"));
    std::thread::sleep(Duration::from_millis(300));
    assert!(Signals::send("-TERM", &server.id().to_string()));
    std::thread::sleep(Duration::from_millis(300));
    assert!(Signals::send("-CONT", &fylo));
    let exited = Signals::exits_within(&mut server, Duration::from_secs(4));
    let written = writer.join().expect("writer thread");
    assert!(exited, "ty exits after SIGTERM");
    let stderr = TimedProject::finish(server);
    assert!(written.starts_with("HTTP/1.1 200"), "{written}\n{stderr}");
    Signals::assert_closed_at_shutdown(&stderr);
    assert_eq!(Signals::stored_events(&project.root()), 1, "{stderr}");
}
