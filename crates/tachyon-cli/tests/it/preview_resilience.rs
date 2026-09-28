#![allow(clippy::expect_used)]
use std::io::{BufRead as _, Read as _, Write as _};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct RunningPreview {
    child: Child,
    output: Arc<Mutex<String>>,
    errors: Arc<Mutex<String>>,
    address: String,
}

impl RunningPreview {
    const WAIT: Duration = Duration::from_mins(1);
    const BROKEN_CONFIG: &'static str = "{ \"tac\": {} }\n";
    const GOOD_CONFIG: &'static str = "{}\n";

    fn start(project: &Path) -> Self {
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|listener| listener.local_addr())
            .expect("free port")
            .port();
        let mut child = Command::new(env!("CARGO_BIN_EXE_ty"))
            .arg("preview")
            .arg(project)
            .args(["--port", &port.to_string(), "--no-self-restart"])
            .env("TAC_ALLOW_UNTESTED", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("ty preview");
        let output = Self::collect(child.stdout.take().expect("stdout"));
        let errors = Self::collect(child.stderr.take().expect("stderr"));
        let preview = Self {
            child,
            output,
            errors,
            address: format!("127.0.0.1:{port}"),
        };
        preview.wait_for_text(&preview.output, "Tachyon preview ready at");
        preview
    }

    fn project() -> tempfile::TempDir {
        let project = tempfile::tempdir().expect("project");
        Self::write(project.path(), "client/pages/tac.html", "<app-view />");
        Self::write(
            project.path(),
            "client/components/app/view/tac.html",
            "<main aria-label=\"Resilient\"><h1>First</h1></main>",
        );
        Self::write(project.path(), "server/application.json", Self::GOOD_CONFIG);
        project
    }

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folders");
        std::fs::write(path, source).expect("source");
    }

    fn collect(stream: impl std::io::Read + Send + 'static) -> Arc<Mutex<String>> {
        let text = Arc::new(Mutex::new(String::new()));
        let written = Arc::clone(&text);
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stream)
                .lines()
                .map_while(Result::ok)
            {
                let mut text = written.lock().expect("collected text");
                text.push_str(&line);
                text.push('\n');
            }
        });
        text
    }

    fn wait_for_text(&self, text: &Mutex<String>, expected: &str) {
        let deadline = Instant::now() + Self::WAIT;
        while !text.lock().expect("text").contains(expected) {
            assert!(
                Instant::now() < deadline,
                "no {expected:?} within {:?}: {}{}",
                Self::WAIT,
                self.output.lock().expect("stdout"),
                self.errors.lock().expect("stderr")
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn get(&self, path: &str) -> String {
        let Ok(mut stream) = std::net::TcpStream::connect(&self.address) else {
            return String::new();
        };
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
        let _ = stream.write_all(request.as_bytes());
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response);
        response
    }

    fn wait_for_response(&self, path: &str, expected: &str) -> String {
        let deadline = Instant::now() + Self::WAIT;
        loop {
            let response = self.get(path);
            if response.contains(expected) {
                return response;
            }
            assert!(
                Instant::now() < deadline,
                "{path} never answered {expected:?}; last answer: {response}\n{}",
                self.errors.lock().expect("stderr")
            );
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    fn is_running(&mut self) -> bool {
        self.child.try_wait().expect("preview status").is_none()
    }
}

impl Drop for RunningPreview {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn a_broken_config_after_start_keeps_the_last_good_site_and_shows_the_problem() {
    let project = RunningPreview::project();
    let mut preview = RunningPreview::start(project.path());
    preview.wait_for_response("/", "First");

    RunningPreview::write(
        project.path(),
        "server/application.json",
        RunningPreview::BROKEN_CONFIG,
    );
    let shown = preview.wait_for_response("/.tachyon/diagnostics", "\"code\":\"TY2010\"");
    assert!(shown.contains("configure Yon only"), "{shown}");
    preview.wait_for_text(&preview.errors, "TY2010");
    assert!(
        preview.get("/").contains("First"),
        "the last good site is gone"
    );

    RunningPreview::write(
        project.path(),
        "server/application.json",
        RunningPreview::GOOD_CONFIG,
    );
    preview.wait_for_response("/.tachyon/diagnostics", "\r\n\r\n[]");
    RunningPreview::write(
        project.path(),
        "client/components/app/view/tac.html",
        "<main><logic :else>orphan</logic></main>",
    );
    preview.wait_for_response("/.tachyon/diagnostics", "\"code\":\"TY");
    assert!(
        preview.get("/").contains("First"),
        "a failed build replaced the site"
    );

    RunningPreview::write(
        project.path(),
        "client/components/app/view/tac.html",
        "<main aria-label=\"Resilient\"><h1>Second</h1></main>",
    );
    preview.wait_for_response("/", "Second");
    preview.wait_for_response("/.tachyon/diagnostics", "\r\n\r\n[]");
    assert!(preview.is_running());
}

#[test]
fn a_broken_project_at_start_serves_an_error_page_until_it_is_fixed() {
    let project = RunningPreview::project();
    RunningPreview::write(
        project.path(),
        "server/application.json",
        RunningPreview::BROKEN_CONFIG,
    );
    let mut preview = RunningPreview::start(project.path());
    let page = preview.wait_for_response("/", "Tachyon cannot start this project");
    assert!(page.contains("503"), "{page}");
    assert!(page.contains("TY2010"), "{page}");
    assert!(page.contains("/.tachyon/live-reload.js"), "{page}");
    let shown = preview.get("/.tachyon/diagnostics");
    assert!(shown.contains("\"code\":\"TY"), "{shown}");
    assert!(preview.is_running());

    RunningPreview::write(
        project.path(),
        "server/application.json",
        RunningPreview::GOOD_CONFIG,
    );
    preview.wait_for_response("/", "First");
    assert!(preview.is_running());
}
