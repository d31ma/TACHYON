#![allow(clippy::expect_used)]

use super::yon_cpp::{CppProject, Exchange};
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

struct AnswerProject;

impl AnswerProject {
    const CONTROLLER: &'static str = r#"#pragma once
#include <string>
#include <vector>

class [[RequestRoute("/texts")]] [[Public]] TextController {
public:
    [[GetRoute("/broken")]]
    std::vector<std::string> getBroken() {
        return {std::string("caf\xE9 au lait"), std::string("half \xE2\x82")};
    }

    [[GetRoute("/raw")]]
    tachyon::http::HttpResponseMessage getRaw() {
        tachyon::http::HttpResponseMessage response;
        response.headers.add("Content-Type", "application/octet-stream");
        response.content = tachyon::http::HttpContent(std::string("\x89PNG\xFF\x00!", 7));
        return response;
    }

    [[PostRoute("/deep")]]
    int postDeep([[FromBody]] tachyon::json::JsonNode body) {
        int depth = 0;
        const tachyon::json::JsonNode* at = &body;
        while (at->count() > 0) {
            at = &at->items()[0];
            depth += 1;
        }
        return depth;
    }
};
"#;

    fn nested(depth: usize) -> String {
        format!("{}{}", "[".repeat(depth), "]".repeat(depth))
    }

    async fn raw(port: u16, path: &str) -> Vec<u8> {
        let mut stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).await.expect("request");
        let mut bytes = Vec::new();
        tokio::time::timeout(Duration::from_mins(2), stream.read_to_end(&mut bytes))
            .await
            .expect("bounded response")
            .expect("response");
        bytes
    }
}

#[tokio::test]
async fn a_cpp_answer_repairs_broken_text_keeps_raw_bytes_and_refuses_deep_bodies() {
    if !CppProject::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    CppProject::write(
        root.path(),
        "server/controllers/TextController.hpp",
        AnswerProject::CONTROLLER,
    );
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

    let broken = Exchange::without_body(port, "GET", "/texts/broken").await;
    assert!(broken.starts_with("HTTP/1.1 200"), "{broken}");
    assert_eq!(
        Exchange::body(&broken),
        serde_json::json!(["caf\u{FFFD} au lait", "half \u{FFFD}\u{FFFD}"])
    );

    let raw = AnswerProject::raw(port, "/texts/raw").await;
    assert!(raw.starts_with(b"HTTP/1.1 200"), "{raw:?}");
    assert!(raw.ends_with(b"\r\n\r\n\x89PNG\xFF\x00!"), "{raw:?}");

    let shallow = Exchange::with_body(port, "/texts/deep", &AnswerProject::nested(10)).await;
    assert!(shallow.starts_with("HTTP/1.1 200"), "{shallow}");
    assert_eq!(Exchange::body(&shallow), serde_json::json!(9));

    let deep = Exchange::with_body(port, "/texts/deep", &AnswerProject::nested(100)).await;
    assert!(deep.starts_with("HTTP/1.1 400"), "{deep}");

    let _ = stop.send(());
    running.await.expect("server task").expect("server");
}
