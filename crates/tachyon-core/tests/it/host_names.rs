#![allow(clippy::expect_used)]

use std::fs;
use std::future::Future;
use std::path::Path;
use std::time::Duration;
use tachyon_core::{
    DevServer, DevServerOptions, ErrorPageServer, Failure, PreviewServer, PreviewServerOptions,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, contents).expect("file");
}

async fn exchange(port: u16, raw: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    stream.write_all(raw.as_bytes()).await.expect("write");
    let mut answer = Vec::new();
    tokio::time::timeout(Duration::from_mins(1), stream.read_to_end(&mut answer))
        .await
        .expect("answer within the deadline")
        .expect("read");
    String::from_utf8_lossy(&answer).into_owned()
}

async fn exchange_until_refused(port: u16, raw: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    stream.write_all(raw.as_bytes()).await.expect("write");
    let mut answer = Vec::new();
    let mut chunk = vec![0_u8; 4096];
    while !String::from_utf8_lossy(&answer).contains("server.hosts") {
        let read = tokio::time::timeout(Duration::from_mins(1), stream.read(&mut chunk))
            .await
            .expect("answer within the deadline")
            .expect("read");
        if read == 0 {
            break;
        }
        answer.extend_from_slice(&chunk[..read]);
    }
    String::from_utf8_lossy(&answer).into_owned()
}

async fn ask(port: u16, path: &str, host: &str) -> String {
    let raw = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    exchange(port, &raw).await
}

fn status_line(answer: &str) -> &str {
    answer.lines().next().unwrap_or_default()
}

fn assert_status(answer: &str, status: &str, host: &str) {
    assert!(
        status_line(answer).contains(status),
        "Host {host}: expected {status}, got {answer}"
    );
}

fn assert_refused(answer: &str, host: &str) {
    assert_status(answer, "403", host);
    assert!(answer.contains("TY1366"), "Host {host}: {answer}");
    assert!(answer.contains("server.hosts"), "Host {host}: {answer}");
}

fn spawn_until_stopped<F>(serve: impl FnOnce(oneshot::Receiver<()>) -> F) -> oneshot::Sender<()>
where
    F: Future<Output = Result<(), Failure>> + Send + 'static,
{
    let (stop, wait) = oneshot::channel::<()>();
    let running = serve(wait);
    tokio::spawn(running);
    stop
}

#[tokio::test]
async fn the_preview_server_answers_only_local_names_addresses_and_the_site_origin() {
    let bundle = tempfile::tempdir().expect("bundle");
    write(&bundle.path().join("index.html"), "<main>Published</main>");
    write(
        &bundle.path().join("manifest.webmanifest"),
        r#"{"name":"Shop","start_url":"/","tac":{"origin":"https://shop.example"}}"#,
    );
    let options = PreviewServerOptions {
        port: 0,
        ..PreviewServerOptions::default()
    };
    let server = PreviewServer::bind(bundle.path(), &options)
        .await
        .expect("preview");
    let port = server.address().port();
    let stop = spawn_until_stopped(|wait| {
        server.run_until(async {
            let _stopped = wait.await;
        })
    });

    let allowed = [
        format!("127.0.0.1:{port}"),
        String::from("127.0.0.2"),
        format!("[::1]:{port}"),
        String::from("localhost"),
        format!("shop.localhost:{port}"),
        String::from("SHOP.LocalHost."),
        String::from("shop.example"),
    ];
    for host in &allowed {
        assert_status(&ask(port, "/", host).await, "200", host);
    }
    let without_host = exchange(port, "GET / HTTP/1.0\r\n\r\n").await;
    assert_status(&without_host, "200", "(none)");

    let refused = [
        String::from("evil.example"),
        format!("evil.example:{port}"),
        String::from("localhost.evil.example"),
        String::from("shop.example.evil.example"),
        String::from("notlocalhost"),
    ];
    for host in &refused {
        assert_refused(&ask(port, "/", host).await, host);
    }
    for host in ["bad host", "user@localhost"] {
        assert_status(&ask(port, "/", host).await, "400", host);
    }
    let _stopped = stop.send(());
}

fn write_served_project(root: &Path) {
    write(&root.join("client/pages/tac.html"), "<host-view />");
    write(
        &root.join("client/components/host/view/tac.html"),
        "<main aria-label=\"Home\"><h1>Home</h1></main>",
    );
    write(
        &root.join("server/controllers/ItemsController.js"),
        r"
@RequestRoute('/items/{id}')
@Public
export class ItemsController {
  @GetRoute
  get(@FromPath id) {
    return { id }
  }
}
",
    );
    write(
        &root.join("server/application.json"),
        r#"{"server":{"hosts":["app.example.test","*.tunnel.example"]}}"#,
    );
}

#[tokio::test]
async fn the_development_server_refuses_foreign_host_names_before_routing() {
    let project = tempfile::tempdir().expect("project");
    write_served_project(project.path());
    let options = DevServerOptions {
        port: 0,
        watch: false,
        ..DevServerOptions::default()
    };
    let server = DevServer::bind(project.path(), &options)
        .await
        .expect("bind");
    let port = server.address().port();
    let stop = spawn_until_stopped(|wait| {
        server.run_until(async {
            let _stopped = wait.await;
        })
    });

    for host in [
        "127.0.0.1",
        "app.example.test",
        "preview.tunnel.example:443",
    ] {
        assert_status(&ask(port, "/", host).await, "200", host);
    }
    let route = ask(port, "/items/7", "app.example.test").await;
    assert_status(&route, "200", "app.example.test");

    for path in ["/", "/items/7", "/.tachyon/hot", "/.tachyon/live-reload.js"] {
        assert_refused(&ask(port, path, "evil.example").await, "evil.example");
    }
    let upgrade = exchange_until_refused(
        port,
        "GET /items/7 HTTP/1.1\r\nHost: evil.example\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nOrigin: http://evil.example\r\n\r\n",
    )
    .await;
    assert_refused(&upgrade, "evil.example (WebSocket upgrade)");
    let _stopped = stop.send(());
}

#[tokio::test]
async fn the_error_page_server_refuses_foreign_host_names() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<broken-view />",
    );
    let options = DevServerOptions {
        port: 0,
        ..DevServerOptions::default()
    };
    let server = ErrorPageServer::bind(project.path(), &options)
        .await
        .expect("error page");
    let port = server.address().port();
    assert_status(&ask(port, "/", "localhost").await, "503", "localhost");
    assert_refused(&ask(port, "/", "evil.example").await, "evil.example");
    server.stop().await;
}
