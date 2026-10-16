#![allow(clippy::expect_used)]

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

struct CheckedOrders;

impl CheckedOrders {
    const FILES: [(&'static str, &'static str); 3] = [
        (
            "server/models/OrderInput.java",
            "@Checked\nclass OrderInput {\n    @NotEmpty\n    @Length({\"min\":2,\"max\":8})\n    String name;\n    @Required\n    @Min(1)\n    @Max(9)\n    Object level;\n}\n",
        ),
        (
            "server/services/OrderService.java",
            "@Service\nclass OrderService {\n    Object accept(@Check(OrderInput.class) Object order) { return order; }\n}\n",
        ),
        (
            "server/controllers/OrdersController.java",
            "@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n    @Needs\n    final OrderService orders;\n    @PostRoute\n    Object postOrders(@FromBody Object order) {\n        return orders.accept(order);\n    }\n}\n",
        ),
    ];

    const ACCEPTED: [(&'static str, &'static str); 6] = [
        ("every rule holds", r#"{"name":"ab","level":3}"#),
        ("Length accepts the minimum", r#"{"name":"ab","level":5}"#),
        (
            "Length accepts the maximum",
            r#"{"name":"abcdefgh","level":5}"#,
        ),
        ("Min accepts the bound", r#"{"name":"order","level":1}"#),
        ("Max accepts the bound", r#"{"name":"order","level":9}"#),
        (
            "unknown fields stay",
            r#"{"name":"order","level":2,"sku":"A-1"}"#,
        ),
    ];

    const REJECTED: [(&'static str, &'static str); 10] = [
        ("Required refuses a missing value", r#"{"name":"order"}"#),
        ("Required refuses null", r#"{"name":"order","level":null}"#),
        ("NotEmpty refuses a missing value", r#"{"level":3}"#),
        ("NotEmpty refuses an empty text", r#"{"name":"","level":3}"#),
        ("NotEmpty refuses blank text", r#"{"name":"   ","level":3}"#),
        ("Length refuses a short text", r#"{"name":"a","level":3}"#),
        (
            "Length refuses a long text",
            r#"{"name":"abcdefghi","level":3}"#,
        ),
        (
            "Min refuses a smaller number",
            r#"{"name":"order","level":0}"#,
        ),
        (
            "Max refuses a larger number",
            r#"{"name":"order","level":10}"#,
        ),
        ("Min refuses a text", r#"{"name":"order","level":"3"}"#),
    ];

    const REJECTED_PREFIX: &'static str = "Handler execution failed. Reference: ";

    fn java_available() -> bool {
        let found = ["java", "javac"].iter().all(|program| {
            Command::new(program)
                .arg("-version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        });
        if !found {
            assert!(std::env::var_os("CI").is_none(), "java is required in CI");
            eprintln!("skipping Java checked models: java or javac is unavailable");
        }
        found
    }

    fn write(root: &Path) {
        for (relative, source) in Self::FILES {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
            std::fs::write(path, source).expect("source");
        }
    }

    fn all_bodies() -> Vec<&'static str> {
        let mut bodies = Vec::new();
        for (_, body) in Self::ACCEPTED {
            bodies.push(body);
        }
        for (_, body) in Self::REJECTED {
            bodies.push(body);
        }
        bodies
    }

    fn split(response: &str) -> (&str, &str) {
        let (head, body) = response.split_once("\r\n\r\n").expect("HTTP response");
        let status_line = head.lines().next().expect("status line");
        (status_line, body)
    }

    fn check_accepted(name: &str, sent: &str, response: &str) {
        let (status_line, body) = Self::split(response);
        assert_eq!(status_line, "HTTP/1.1 200 OK", "{name}: {response}");
        assert!(
            response.contains("content-type: application/json"),
            "{name}: {response}"
        );
        let answered: serde_json::Value = serde_json::from_str(body).expect("JSON answer");
        let expected: serde_json::Value = serde_json::from_str(sent).expect("JSON request");
        assert_eq!(answered, expected, "{name}");
    }

    fn check_rejected(name: &str, response: &str) {
        let (status_line, body) = Self::split(response);
        assert_eq!(
            status_line, "HTTP/1.1 500 Internal Server Error",
            "{name}: {response}"
        );
        assert!(
            response.contains("content-type: text/plain; charset=utf-8"),
            "{name}: {response}"
        );
        let reference = body
            .strip_prefix(Self::REJECTED_PREFIX)
            .and_then(|rest| rest.strip_suffix('.'))
            .unwrap_or_else(|| panic!("{name}: {body}"));
        assert!(!reference.is_empty(), "{name}: {body}");
        let reference_is_plain = reference
            .chars()
            .all(|character| character.is_ascii_alphanumeric());
        assert!(reference_is_plain, "{name}: {body}");
    }
}

struct Http;

impl Http {
    async fn post_json(port: u16, path: &str, body: &str) -> String {
        let mut socket = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        let length = body.len();
        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{body}"
        );
        socket.write_all(request.as_bytes()).await.expect("request");
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_mins(2), socket.read_to_end(&mut response))
            .await
            .expect("response deadline")
            .expect("response");
        String::from_utf8(response).expect("UTF-8 response")
    }

    async fn post_all(root: &Path, path: &str, bodies: &[&str]) -> Vec<String> {
        let project = ProjectDiscovery::discover(root).expect("discover");
        let options = DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        };
        let server = DevServer::bind_project(&project, &options)
            .await
            .expect("bind");
        let port = server.address().port();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let running = tokio::spawn(server.run_until(async {
            let _ = stopped.await;
        }));
        let mut responses = Vec::new();
        for body in bodies {
            responses.push(Self::post_json(port, path, body).await);
        }
        stop.send(()).expect("stop");
        running.await.expect("task").expect("shutdown");
        responses
    }
}

#[tokio::test]
async fn a_java_service_checks_a_model_from_server_models_over_http() {
    if !CheckedOrders::java_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    CheckedOrders::write(root.path());
    let bodies = CheckedOrders::all_bodies();
    let responses = Http::post_all(root.path(), "/orders", &bodies).await;
    let (accepted, rejected) = responses.split_at(CheckedOrders::ACCEPTED.len());
    for ((name, sent), response) in CheckedOrders::ACCEPTED.iter().zip(accepted) {
        CheckedOrders::check_accepted(name, sent, response);
    }
    for ((name, _), response) in CheckedOrders::REJECTED.iter().zip(rejected) {
        CheckedOrders::check_rejected(name, response);
    }
}
