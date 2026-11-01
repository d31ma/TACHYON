#![allow(clippy::expect_used)]

use std::fs;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc;
use std::time::Duration;

struct Server(Child);

impl Server {
    const FRAMEWORK_NAMES: [&'static str; 8] = [
        "YON_PROFILE",
        "YON_HOST",
        "YON_HOSTNAME",
        "YON_PORT",
        "YON_SKIP_BUNDLE",
        "HOST",
        "PORT",
        "YON_APPLICATION_JSON",
    ];
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn ty() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
    for name in Server::FRAMEWORK_NAMES {
        command.env_remove(name);
    }
    if cfg!(windows) {
        command.env("YON_EGRESS_ENFORCEMENT", "proxy");
    }
    command
}

fn run(command: &mut Command) -> Output {
    command.output().expect("the ty process should start")
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("fixture folder");
    for entry in fs::read_dir(from).expect("fixture entries").flatten() {
        let target = to.join(entry.file_name());
        if entry.file_type().expect("fixture entry").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("fixture file");
        }
    }
}

fn scaffold() -> tempfile::TempDir {
    let project = tempfile::tempdir().expect("project");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scaffold-26.36.01"),
        project.path(),
    );
    project
}

fn ready_address(server: &mut Server) -> String {
    let pipe = server.0.stdout.take().expect("readiness pipe");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(pipe).lines().map_while(Result::ok) {
            if line.contains(" ready at http://") {
                let _ = sender.send(line);
                break;
            }
        }
    });
    let line = receiver
        .recv_timeout(Duration::from_mins(1))
        .expect("ty start is ready within a minute");
    let address = line
        .split_whitespace()
        .find(|part| part.starts_with("http://"))
        .expect("address");
    String::from(address.trim_start_matches("http://").trim_end_matches('/'))
}

fn get(address: &str, path: &str) -> String {
    let mut stream = TcpStream::connect(address).expect("server connection");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("read timeout");
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
    )
    .expect("request");
    let mut response = String::new();
    stream.read_to_string(&mut response).expect("response");
    response
}

#[test]
fn a_26_36_01_scaffold_upgrades_with_migrate_alone_and_then_bundles_and_starts() {
    let project = scaffold();
    let root = project.path();

    let check = run(ty().args(["migrate", "check"]).arg(root));
    let report = text(&check);
    assert!(check.status.success(), "{report}");
    for expected in [
        "view.page_markup",
        "client/pages/tac.html",
        "companion.page",
        "client/pages/tac.js",
        "environment.env_name",
        ".env.example:1",
        "server/routes/yon.js",
    ] {
        assert!(
            report.contains(expected),
            "check names {expected}: {report}"
        );
    }
    let before = run(ty().arg("bundle").arg(root));
    assert!(
        !before.status.success(),
        "the 26.36.01 scaffold should not bundle as it is"
    );

    let fixed = run(ty().args(["migrate", "fix"]).arg(root).arg("--write"));
    let fix_report = text(&fixed);
    assert!(fixed.status.success(), "{fix_report}");
    for expected in [
        "moved    server/routes/yon.js -> server/controllers/HomeController.js",
        "moved    client/pages/tac.js -> client/shared/scripts/home-page.js",
        "moved    client/pages/tac.css -> client/shared/styles/home-page.css",
    ] {
        assert!(
            fix_report.contains(expected),
            "fix says {expected}: {fix_report}"
        );
    }
    assert!(!root.join("server/routes").exists(), "{fix_report}");
    assert_eq!(
        fs::read_to_string(root.join("client/pages/tac.html")).expect("page"),
        "<page-home />\n"
    );

    let again = run(ty().args(["migrate", "check"]).arg(root));
    let rechecked = text(&again);
    assert!(again.status.success(), "{rechecked}");
    for gone in [
        "view.page_markup",
        "companion.page",
        "environment.env_name",
        "server/routes/yon.js",
    ] {
        assert!(!rechecked.contains(gone), "{gone} is fixed: {rechecked}");
    }

    fs::copy(root.join(".env.example"), root.join(".env")).expect("the README's copy step");
    let bundled = run(ty().arg("bundle").arg(root));
    assert!(bundled.status.success(), "{}", text(&bundled));
    let page = fs::read_to_string(root.join("dist/web/index.html")).expect("published page");
    for linked in [
        "/shared/styles/home-page.css",
        "/shared/scripts/home-page.js",
        "page-home",
    ] {
        assert!(page.contains(linked), "the page links {linked}");
    }

    let mut server = Server(
        ty().arg("start")
            .arg(root)
            .args(["--host", "127.0.0.1", "--port", "0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("ty start"),
    );
    let address = ready_address(&mut server);
    let response = get(&address, "/");
    drop(server);
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let (_, body) = response.split_once("\r\n\r\n").expect("response body");
    let answer: serde_json::Value = serde_json::from_str(body).expect("JSON body");
    assert_eq!(
        answer,
        serde_json::json!({"ok": true, "framework": "Tachyon"})
    );
}
