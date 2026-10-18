#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

struct CppProject;

impl CppProject {
    const MODEL: &'static str = r"#pragma once
#include <optional>
#include <string>

struct Order {
    std::string item;
    int count = 1;
    std::optional<std::string> note;
};
";
    const CONTROLLER: &'static str = r#"#pragma once
#include <stdexcept>
#include <string>
#include <vector>
#include "../models/Order.hpp"

class [[RequestRoute("/orders")]] [[Public]] OrderController {
public:
    [[GetRoute("/{id}")]]
    Order getOrder([[FromPath]] int id, [[FromQuery("view")]] std::optional<std::string> view) {
        Order order;
        order.item = "tea";
        order.count = id;
        order.note = view;
        return order;
    }

    [[PostRoute]]
    std::vector<Order> postOrder([[FromBody]] const Order& order) {
        if (order.count > 9) {
            throw std::runtime_error("too many cups");
        }
        return {order, order};
    }

    [[DeleteRoute("/{id}")]]
    void deleteOrder([[FromPath]] int id) {
        removed += id;
    }

private:
    int removed = 0;
};
"#;

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, source).expect("source");
    }

    fn with_controller(controller: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        Self::write(root.path(), "server/models/Order.hpp", Self::MODEL);
        Self::write(
            root.path(),
            "server/controllers/OrderController.hpp",
            controller,
        );
        root
    }

    fn compiler_available() -> bool {
        std::process::Command::new("clang++")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn refusal(controller: &str) -> String {
        let root = Self::with_controller(controller);
        ProjectDiscovery::discover(root.path())
            .expect_err("the C++ controller is refused")
            .to_string()
    }
}

struct Exchange;

impl Exchange {
    async fn send(port: u16, request: String) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        stream.write_all(request.as_bytes()).await.expect("request");
        let mut bytes = Vec::new();
        tokio::time::timeout(Duration::from_mins(2), stream.read_to_end(&mut bytes))
            .await
            .expect("bounded response")
            .expect("response");
        String::from_utf8(bytes).expect("UTF-8 response")
    }

    async fn without_body(port: u16, method: &str, path: &str) -> String {
        Self::send(
            port,
            format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"),
        )
        .await
    }

    async fn with_body(port: u16, path: &str, body: &str) -> String {
        Self::send(
            port,
            format!(
                "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            ),
        )
        .await
    }

    fn body(response: &str) -> serde_json::Value {
        let (_, body) = response.split_once("\r\n\r\n").expect("response body");
        serde_json::from_str(body).expect("JSON body")
    }
}

#[tokio::test]
async fn a_cpp_controller_answers_typed_routes_through_clang() {
    if !CppProject::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let root = CppProject::with_controller(CppProject::CONTROLLER);
    let project = ProjectDiscovery::discover(root.path()).expect("discovery");
    let server = DevServer::bind_project(
        &project,
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(server.run_until(async {
        let _ = stopped.await;
    }));

    let found = Exchange::without_body(port, "GET", "/orders/7?view=full").await;
    assert!(found.starts_with("HTTP/1.1 200"), "{found}");
    assert_eq!(
        Exchange::body(&found),
        serde_json::json!({"item": "tea", "count": 7, "note": "full"})
    );

    let created = Exchange::with_body(port, "/orders", r#"{"item":"green","count":2}"#).await;
    assert!(created.starts_with("HTTP/1.1 200"), "{created}");
    assert_eq!(
        Exchange::body(&created),
        serde_json::json!([
            {"item": "green", "count": 2, "note": null},
            {"item": "green", "count": 2, "note": null}
        ])
    );

    let refused = Exchange::with_body(port, "/orders", r#"{"count":2}"#).await;
    assert!(refused.starts_with("HTTP/1.1 400"), "{refused}");

    let failed = Exchange::with_body(port, "/orders", r#"{"item":"green","count":20}"#).await;
    assert!(failed.starts_with("HTTP/1.1 500"), "{failed}");
    assert!(!failed.contains("too many cups"), "{failed}");

    let removed = Exchange::without_body(port, "DELETE", "/orders/3").await;
    assert!(removed.starts_with("HTTP/1.1 204"), "{removed}");

    let _ = stop.send(());
    running.await.expect("server task").expect("server");
}

#[test]
fn a_cpp_marker_that_yon_cpp_lacks_is_refused_by_name() {
    let controller = CppProject::CONTROLLER.replace(
        "[[GetRoute(\"/{id}\")]]",
        "[[GetRoute(\"/{id}\")]] [[Cached]]",
    );
    let failure = CppProject::refusal(&controller);
    assert!(failure.contains("TY1308"), "{failure}");
    assert!(failure.contains("[[Cached]]"), "{failure}");
    assert!(failure.contains("C++ gets it in a later step"), "{failure}");
}

#[test]
fn a_cpp_route_parameter_without_a_source_marker_is_refused() {
    let controller = CppProject::CONTROLLER.replace("[[FromPath]] int id)", "int id)");
    let failure = CppProject::refusal(&controller);
    assert!(failure.contains("TY2015"), "{failure}");
    assert!(failure.contains("[[FromPath]], [[FromQuery]]"), "{failure}");
}

#[test]
fn a_cpp_socket_controller_is_refused_until_cpp_sockets_land() {
    let controller = CppProject::CONTROLLER.replace(
        "class [[RequestRoute(\"/orders\")]] [[Public]] OrderController",
        "class [[SocketRoute(\"/orders\")]] [[Public]] OrderController",
    );
    let failure = CppProject::refusal(&controller);
    assert!(failure.contains("[[SocketRoute]]"), "{failure}");
    assert!(failure.contains("C++ gets it in a later step"), "{failure}");
}

#[test]
fn a_cpp_service_role_is_not_yet_a_yon_role() {
    let root = CppProject::with_controller(CppProject::CONTROLLER);
    CppProject::write(
        root.path(),
        "server/services/PriceService.hpp",
        "class [[Service]] PriceService {\npublic:\n    int price() { return 3; }\n};\n",
    );
    let failure = ProjectDiscovery::discover(root.path())
        .expect_err("a C++ Yon service is refused")
        .to_string();
    assert!(failure.contains("[[Service]]"), "{failure}");
    assert!(failure.contains("C++ gets it in a later step"), "{failure}");
}

#[test]
fn a_cpp_compiler_error_names_the_authored_file() {
    if !CppProject::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let controller = CppProject::CONTROLLER.replace("removed += id;", "removed += missingName;");
    let failure = CppProject::refusal(&controller);
    assert!(failure.contains("failed to compile"), "{failure}");
    assert!(
        failure.contains("server/controllers/OrderController.hpp"),
        "{failure}"
    );
    assert!(failure.contains("missingName"), "{failure}");
}
