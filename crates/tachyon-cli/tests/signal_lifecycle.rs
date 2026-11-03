#![allow(clippy::expect_used)]

#[cfg(unix)]
mod unix {
    use serde_json::Value;
    use std::fs;
    use std::io::{BufRead as _, BufReader};
    #[cfg(target_os = "macos")]
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    #[cfg(target_os = "macos")]
    use std::net::TcpStream;
    use std::process::{Child, Command, Stdio};
    use std::sync::Mutex;
    use std::sync::mpsc::{self, Receiver};
    use std::time::{Duration, Instant};

    fn serialize_process_test() -> std::sync::MutexGuard<'static, ()> {
        static PROCESS_TEST: Mutex<()> = Mutex::new(());
        PROCESS_TEST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    struct RunningChild {
        child: Child,
    }

    impl RunningChild {
        fn spawn_command(mut command: Command) -> (Self, Receiver<String>, Receiver<String>) {
            let mut child = command
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("long-running command should start");
            let stdout = lines(child.stdout.take().expect("command stdout"));
            let stderr = lines(child.stderr.take().expect("command stderr"));
            (Self { child }, stdout, stderr)
        }

        fn spawn(
            project: &std::path::Path,
            watch: bool,
        ) -> (Self, Receiver<String>, Receiver<String>) {
            let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
            command
                .arg("serve")
                .arg(project)
                .args(["--host", "127.0.0.1", "--port", "0"]);
            if !watch {
                command.arg("--no-watch");
            }
            Self::spawn_command(command)
        }

        fn signal(&self, signal: &str) {
            let status = Command::new("kill")
                .arg(format!("-{signal}"))
                .arg(self.child.id().to_string())
                .status()
                .expect("the Unix kill utility should run");
            assert!(status.success(), "signal should be delivered");
        }

        fn wait_bounded(&mut self) -> std::process::ExitStatus {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                if let Some(status) = self.child.try_wait().expect("child status") {
                    return status;
                }
                assert!(Instant::now() < deadline, "server did not stop within 10s");
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }

    impl Drop for RunningChild {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    fn lines<R>(reader: R) -> Receiver<String>
    where
        R: std::io::Read + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(reader).lines() {
                let Ok(line) = line else {
                    return;
                };
                if sender.send(line).is_err() {
                    return;
                }
            }
        });
        receiver
    }

    fn project() -> tempfile::TempDir {
        let project = tempfile::tempdir().expect("project directory");
        let page = project.path().join("client/pages/tac.html");
        fs::create_dir_all(page.parent().expect("page parent")).expect("page directory");
        fs::write(page, "<app-view />").expect("page source");
        let component = project.path().join("client/components/app/view/tac.html");
        fs::create_dir_all(component.parent().expect("component parent"))
            .expect("component directory");
        fs::write(
            component,
            "<main aria-label=\"Signal\"><h1>Signal</h1></main>",
        )
        .expect("component source");
        project
    }

    fn event(receiver: &Receiver<String>) -> Value {
        let line = receiver
            .recv_timeout(Duration::from_secs(30))
            .expect("lifecycle event within 30s");
        serde_json::from_str(&line).expect("lifecycle event should be JSON")
    }

    fn line_containing(receiver: &Receiver<String>, expected: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let line = receiver
                .recv_timeout(remaining)
                .unwrap_or_else(|_| panic!("output containing {expected:?} within 30s"));
            if line.contains(expected) {
                return line;
            }
        }
    }

    fn assert_graceful_signal(delivery_name: &str, event_name: &str, event_code: i64) {
        let project = project();
        let (mut server, stdout, stderr) = RunningChild::spawn(project.path(), false);

        let startup = event(&stderr);
        assert_eq!(startup["event"], "runtime.signal_handlers_ready");
        assert_eq!(startup["command"], "serve");
        assert_eq!(
            startup["installed"],
            serde_json::json!(["SIGINT", "SIGTERM"])
        );

        let ready = stdout
            .recv_timeout(Duration::from_secs(10))
            .expect("readiness line within 10s");
        assert!(ready.contains("Tachyon server ready at http://127.0.0.1:"));

        server.signal(delivery_name);
        let receipt = event(&stderr);
        assert_eq!(receipt["event"], "runtime.shutdown_requested");
        assert_eq!(receipt["signal"], event_name);
        assert_eq!(receipt["signal_code"], event_code);
        assert!(server.wait_bounded().success());
    }

    #[test]
    fn sigterm_is_reported_and_stops_the_server_gracefully() {
        let _serial = serialize_process_test();
        assert_graceful_signal("TERM", "SIGTERM", 15);
    }

    #[test]
    fn sigint_is_reported_and_stops_the_server_gracefully() {
        let _serial = serialize_process_test();
        assert_graceful_signal("INT", "SIGINT", 2);
    }

    fn assert_command_shutdown(command: Command, command_name: &str, announcement: &str) {
        let (mut running, stdout, stderr) = RunningChild::spawn_command(command);
        let startup = event(&stderr);
        assert_eq!(startup["event"], "runtime.signal_handlers_ready");
        assert_eq!(startup["command"], command_name);
        let ready = stdout
            .recv_timeout(Duration::from_secs(10))
            .expect("command announcement within 10s");
        assert!(ready.contains(announcement), "{ready}");
        running.signal("TERM");
        let receipt = event(&stderr);
        assert_eq!(receipt["command"], command_name);
        assert_eq!(receipt["signal"], "SIGTERM");
        assert!(running.wait_bounded().success());
    }

    #[test]
    fn bundle_watch_uses_the_shared_signal_lifecycle() {
        let _serial = serialize_process_test();
        let project = project();
        let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
        command
            .arg("bundle")
            .arg(project.path())
            .args(["--watch", "--skip-initial-build"]);
        assert_command_shutdown(command, "bundle.watch", "Watching sources");
    }

    #[test]
    fn production_start_uses_the_shared_signal_lifecycle() {
        let _serial = serialize_process_test();
        let project = project();
        let bundled = Command::new(env!("CARGO_BIN_EXE_ty"))
            .arg("bundle")
            .arg(project.path())
            .output()
            .expect("bundle command should run");
        assert!(
            bundled.status.success(),
            "{}",
            String::from_utf8_lossy(&bundled.stderr)
        );
        let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
        command
            .arg("start")
            .arg(project.path())
            .args(["--host", "127.0.0.1", "--port", "0"]);
        assert_command_shutdown(command, "start", "Tachyon production server ready");
    }

    #[test]
    fn preview_uses_the_shared_signal_lifecycle() {
        let _serial = serialize_process_test();
        let project = project();
        let bundled = Command::new(env!("CARGO_BIN_EXE_ty"))
            .arg("bundle")
            .arg(project.path())
            .output()
            .expect("bundle command should run");
        assert!(
            bundled.status.success(),
            "{}",
            String::from_utf8_lossy(&bundled.stderr)
        );
        let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
        command
            .arg("preview")
            .arg(project.path())
            .args(["--host", "127.0.0.1", "--port", "0"]);
        assert_command_shutdown(command, "preview", "Tachyon preview ready");
    }

    #[test]
    fn preview_watch_cancels_and_joins_its_rebuilder_before_exit() {
        let _serial = serialize_process_test();
        let project = project();
        let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
        command.arg("preview").arg(project.path()).args([
            "--static",
            "--watch",
            "--host",
            "127.0.0.1",
            "--port",
            "0",
        ]);
        let (mut running, stdout, stderr) = RunningChild::spawn_command(command);

        let startup = event(&stderr);
        assert_eq!(startup["event"], "runtime.signal_handlers_ready");
        assert_eq!(startup["command"], "preview");
        let ready = line_containing(&stdout, "Tachyon preview ready");
        let address = ready
            .split_whitespace()
            .find(|part| part.starts_with("http://"))
            .expect("readiness address")
            .trim_end_matches('/')
            .trim_start_matches("http://")
            .to_owned();
        let announcement = line_containing(&stdout, "Watching sources");
        assert!(announcement.contains("rebuilding the preview"));
        fs::write(
            project.path().join("client/components/app/view/tac.html"),
            "<main><h1>Watcher rebuild before shutdown</h1></main>",
        )
        .expect("pre-signal source change");
        let rebuilt = line_containing(&stdout, "Built");
        assert!(rebuilt.contains("dist/web"), "{rebuilt}");
        let published_path = project.path().join("dist/web/index.html");
        let published = fs::read(&published_path).expect("initial preview bundle");
        assert!(
            String::from_utf8_lossy(&published).contains("Watcher rebuild before shutdown"),
            "the watcher should publish its pre-signal rebuild"
        );

        running.signal("TERM");
        let receipt = event(&stderr);
        assert_eq!(receipt["event"], "runtime.shutdown_requested");
        assert_eq!(receipt["command"], "preview");
        assert_eq!(receipt["signal"], "SIGTERM");
        fs::write(
            project.path().join("client/components/app/view/tac.html"),
            "<main><h1>Must not publish after shutdown</h1></main>",
        )
        .expect("post-signal source change");

        assert!(running.wait_bounded().success());
        let listener = TcpListener::bind(&address).expect("preview listener should be released");
        drop(listener);
        std::thread::sleep(Duration::from_millis(600));
        assert_eq!(
            fs::read(published_path).expect("published preview should remain readable"),
            published,
            "the joined watcher must not publish a post-signal rebuild"
        );
    }

    #[cfg(target_os = "macos")]
    fn ready_address(receiver: &Receiver<String>) -> String {
        let ready = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("readiness line within 10s");
        ready
            .split_whitespace()
            .find(|part| part.starts_with("http://"))
            .expect("readiness address")
            .trim_end_matches('/')
            .trim_start_matches("http://")
            .to_owned()
    }

    #[cfg(target_os = "macos")]
    fn probe(address: &str) {
        let mut stream = TcpStream::connect(address).expect("server should accept a probe");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("probe read timeout");
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .expect("probe request");
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("probe response");
        assert!(response.contains("200 OK"), "{response}");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn three_supervised_servers_remain_healthy_for_sixty_seconds() {
        let _serial = serialize_process_test();
        let projects = (0..3).map(|_| project()).collect::<Vec<_>>();
        let mut servers = Vec::new();
        for project in &projects {
            let (server, stdout, stderr) = RunningChild::spawn(project.path(), true);
            let startup = event(&stderr);
            assert_eq!(startup["event"], "runtime.signal_handlers_ready");
            let address = ready_address(&stdout);
            probe(&address);
            servers.push((server, stderr, address));
        }

        let started = Instant::now();
        let mut midpoint_probed = false;
        while started.elapsed() < Duration::from_mins(1) {
            for (server, _, _) in &mut servers {
                assert!(
                    server.child.try_wait().expect("server status").is_none(),
                    "supervised server exited during idle observation"
                );
            }
            if !midpoint_probed && started.elapsed() >= Duration::from_secs(30) {
                for (_, _, address) in &servers {
                    probe(address);
                }
                midpoint_probed = true;
            }
            std::thread::sleep(Duration::from_millis(250));
        }

        for (server, stderr, address) in &mut servers {
            probe(address);
            server.signal("TERM");
            let receipt = event(stderr);
            assert_eq!(receipt["signal"], "SIGTERM");
            assert!(server.wait_bounded().success());
        }
    }

    fn replace_executable(executable: &std::path::Path, contents: &[u8]) {
        use std::os::unix::fs::PermissionsExt as _;
        let next = executable.with_extension("next");
        fs::write(&next, contents).expect("new executable");
        fs::set_permissions(&next, fs::Permissions::from_mode(0o755)).expect("executable mode");
        fs::rename(&next, executable).expect("replace executable");
    }

    fn start_copied_preview(
        executable: &std::path::Path,
        project: &std::path::Path,
    ) -> (RunningChild, Receiver<String>, Receiver<String>, String) {
        replace_executable(
            executable,
            &fs::read(env!("CARGO_BIN_EXE_ty")).expect("ty binary"),
        );
        let port = TcpListener::bind("127.0.0.1:0")
            .and_then(|listener| listener.local_addr())
            .expect("free port")
            .port();
        let mut command = Command::new(executable);
        command.arg("preview").arg(project).args([
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ]);
        let (server, stdout, stderr) = RunningChild::spawn_command(command);
        line_containing(&stdout, "Tachyon preview ready");
        (server, stdout, stderr, format!("127.0.0.1:{port}"))
    }

    fn open_hot_updates(address: &str, last_event_id: Option<&str>) -> std::net::TcpStream {
        use std::io::Write as _;
        let mut stream = std::net::TcpStream::connect(address).expect("hot-update connection");
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("hot-update read timeout");
        let resume = last_event_id
            .map(|id| format!("Last-Event-ID: {id}\r\n"))
            .unwrap_or_default();
        write!(
            stream,
            "GET /.tachyon/hot HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept: text/event-stream\r\n{resume}\r\n"
        )
        .expect("hot-update request");
        stream
    }

    fn read_until(stream: &mut std::net::TcpStream, expected: &str) -> String {
        use std::io::Read as _;
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut seen = String::new();
        let mut buffer = [0_u8; 4096];
        while !seen.contains(expected) {
            assert!(
                Instant::now() < deadline,
                "no {expected:?} within 30s: {seen}"
            );
            if let Ok(read) = stream.read(&mut buffer) {
                seen.push_str(&String::from_utf8_lossy(&buffer[..read]));
            }
        }
        seen
    }

    fn event_id_before(stream_text: &str, expected: &str) -> String {
        let message = stream_text.find(expected).expect("message in stream");
        let id = stream_text[..message].rfind("id: ").expect("event id") + 4;
        stream_text[id..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect()
    }

    fn page_status(address: &str) -> String {
        use std::io::{Read as _, Write as _};
        let mut stream = std::net::TcpStream::connect(address).expect("page connection");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("page read timeout");
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .expect("page request");
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response);
        response.lines().next().unwrap_or_default().to_owned()
    }

    #[test]
    fn preview_restarts_itself_when_ty_changes_on_disk() {
        let _serial = serialize_process_test();
        let project = project();
        let tools = tempfile::tempdir().expect("tool directory");
        let executable = tools.path().join("ty");
        let (_server, stdout, _stderr, address) = start_copied_preview(&executable, project.path());
        let mut updates = open_hot_updates(&address, None);
        read_until(&mut updates, ": connected");

        replace_executable(
            &executable,
            &fs::read(env!("CARGO_BIN_EXE_ty")).expect("ty binary"),
        );
        let notice = read_until(&mut updates, "Restarting the preview");
        assert!(notice.contains(r#""kind":"diagnostics""#), "{notice}");
        line_containing(&stdout, "Restarting the preview");
        line_containing(&stdout, "Tachyon preview ready");

        let last_seen = event_id_before(&notice, "Restarting the preview");
        let mut reconnected = open_hot_updates(&address, Some(&last_seen));
        read_until(&mut reconnected, r#""kind":"reload""#);
        assert!(page_status(&address).contains("200"));
    }

    #[test]
    fn preview_keeps_running_when_the_new_ty_does_not_start() {
        let _serial = serialize_process_test();
        let project = project();
        let tools = tempfile::tempdir().expect("tool directory");
        let executable = tools.path().join("ty");
        let (_server, _stdout, _stderr, address) =
            start_copied_preview(&executable, project.path());
        let mut updates = open_hot_updates(&address, None);
        read_until(&mut updates, ": connected");

        replace_executable(&executable, b"#!/bin/sh\nexit 3\n");
        let notice = read_until(&mut updates, "does not start");
        assert!(
            notice.contains("keeps running the previous version"),
            "{notice}"
        );
        assert!(page_status(&address).contains("200"));
    }

    #[test]
    fn preview_keeps_the_old_server_when_the_new_ty_refuses_the_project() {
        let _serial = serialize_process_test();
        let project = project();
        let tools = tempfile::tempdir().expect("tool directory");
        let executable = tools.path().join("ty");
        let (mut server, _stdout, _stderr, address) =
            start_copied_preview(&executable, project.path());
        let mut updates = open_hot_updates(&address, None);
        read_until(&mut updates, ": connected");

        replace_executable(
            &executable,
            b"#!/bin/sh\ncase \"$*\" in *--check-project*) echo 'error[TY1333]: this project breaks a new rule' >&2; exit 1;; esac\nexit 0\n",
        );
        let notice = read_until(&mut updates, "keeps running the previous version");
        assert!(notice.contains("TY1333"), "{notice}");
        assert!(page_status(&address).contains("200"));
        assert!(server.child.try_wait().expect("preview status").is_none());
    }
}

#[cfg(windows)]
#[test]
fn ctrl_break_is_reported_and_stops_the_server_gracefully() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/windows/signal-lifecycle.py");
    let output = std::process::Command::new("python")
        .arg(script)
        .arg(env!("CARGO_BIN_EXE_ty"))
        .output()
        .expect("Windows signal lifecycle script should run");
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(windows)]
#[test]
fn preview_restarts_itself_as_a_child_when_ty_changes_on_disk() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/windows/self-restart.py");
    let output = std::process::Command::new("python")
        .arg(script)
        .arg(env!("CARGO_BIN_EXE_ty"))
        .output()
        .expect("Windows self-restart script should run");
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
