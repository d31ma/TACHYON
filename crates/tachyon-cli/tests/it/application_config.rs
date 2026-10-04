#![allow(clippy::expect_used)]

use std::fs;
use std::io::{BufRead as _, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

struct Process(Child);

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn command(project: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
    command.arg("start").arg(project);
    for name in [
        "YON_PROFILE",
        "YON_HOST",
        "YON_HOSTNAME",
        "HOST",
        "YON_PORT",
        "PORT",
        "YON_DATASOURCE_URL",
        "YON_DATASOURCE_USERNAME",
        "YON_DATASOURCE_PASSWORD",
        "YON_REDIS_URL",
    ] {
        command.env_remove(name);
    }
    command
}

fn ready(mut command: Command) -> String {
    let mut process = Process(
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start CLI"),
    );
    let output = process.0.stdout.take().expect("stdout");
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut line = String::new();
        BufReader::new(output)
            .read_line(&mut line)
            .expect("read readiness");
        let _ = sender.send(line);
    });
    let line = receiver.recv_timeout(Duration::from_secs(30));
    drop(process);
    reader.join().expect("readiness thread");
    let line = line.expect("bounded readiness");
    assert!(line.contains("Tachyon production server ready"), "{line}");
    line
}

fn fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().expect("project");
    fs::create_dir_all(project.path().join("dist/web")).expect("bundle");
    fs::create_dir_all(project.path().join("client/pages")).expect("pages");
    fs::create_dir_all(project.path().join("server")).expect("server");
    fs::write(
        project.path().join("client/pages/tac.html"),
        "<test-view1 />",
    )
    .expect("page");
    fs::create_dir_all(project.path().join("client/components/test/view1")).expect("component");
    fs::write(
        project.path().join("client/components/test/view1/tac.html"),
        "<main>Ready</main>",
    )
    .expect("view");
    fs::write(
        project.path().join("dist/web/index.html"),
        "<main>Ready</main>",
    )
    .expect("output");
    project
}

#[test]
fn file_profile_and_cli_resolve_before_server_binding() {
    let project = fixture();
    fs::write(
        project.path().join("server/application.json"),
        r#"{"server":{"host":"127.0.0.1","port":0}}"#,
    )
    .expect("base");
    assert!(ready(command(project.path())).contains("http://127.0.0.1:"));

    fs::write(
        project.path().join("server/application.json"),
        r#"{"server":{"host":"invalid-host","port":"invalid-port"}}"#,
    )
    .expect("base endpoints");
    fs::write(
        project.path().join("server/application-test.json"),
        r#"{"server":{"host":"localhost","port":0}}"#,
    )
    .expect("profile");
    let mut profile = command(project.path());
    profile.env("YON_PROFILE", "test");
    assert!(ready(profile).contains("http://127.0.0.1:"));

    let mut environment = command(project.path());
    environment
        .env("YON_HOST", "127.0.0.1")
        .env("YON_PORT", "0");
    let output = environment.output().expect("ignored parent endpoints");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("TY2010"));

    let mut explicit = command(project.path());
    explicit
        .env("YON_HOST", "invalid-lower-priority")
        .env("YON_PORT", "invalid-lower-priority")
        .args(["--host", "127.0.0.1", "--port", "0"]);
    assert!(ready(explicit).contains("http://127.0.0.1:"));
}

#[test]
fn configuration_cannot_enable_public_binding_or_leak_invalid_values() {
    let project = fixture();
    fs::write(
        project.path().join("server/application.json"),
        r#"{"server":{"host":"0.0.0.0","port":0}}"#,
    )
    .expect("public host");
    let output = command(project.path())
        .output()
        .expect("public binding rejection");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("TY1301"));

    fs::write(
        project.path().join("server/application.json"),
        r#"{"datasource":{"password":"secret-canary"},"unexpected":"secret-canary"}"#,
    )
    .expect("invalid config");
    let output = command(project.path())
        .output()
        .expect("configuration rejection");
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("secret-canary"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("secret-canary"));

    fs::write(project.path().join("server/application.json"), "{}").expect("empty config");
    fs::write(project.path().join(".env"), "TAC_TOKEN=secret-canary")
        .expect("invalid client secret");
    let output = command(project.path())
        .output()
        .expect("secret boundary rejection");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("TY2010"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("secret-canary"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("secret-canary"));
}

#[test]
fn a_name_under_localhost_binds_loopback_and_prints_the_name() {
    let project = fixture();
    let mut explicit = command(project.path());
    explicit.args(["--host", "shop.localhost", "--port", "0"]);
    assert!(ready(explicit).contains("http://shop.localhost:"));

    fs::write(
        project.path().join("server/application-test.json"),
        r#"{"server":{"host":"Admin.Localhost","port":0}}"#,
    )
    .expect("profile");
    let mut profile = command(project.path());
    profile.env("YON_PROFILE", "test");
    assert!(ready(profile).contains("http://admin.localhost:"));

    for host in ["example.com", "a..localhost", ".localhost"] {
        let output = command(project.path())
            .args(["--host", host, "--port", "0"])
            .output()
            .expect("host rejection");
        assert!(!output.status.success(), "{host}");
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(diagnostic.contains("TY1302"), "{host}: {diagnostic}");
        assert!(
            diagnostic.contains("a name under .localhost"),
            "{diagnostic}"
        );
    }
}
