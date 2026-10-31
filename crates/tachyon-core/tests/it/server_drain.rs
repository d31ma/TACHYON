use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, contents).expect("source");
}

fn write_slow_project(root: &Path) {
    write(&root.join("client/pages/tac.html"), "<test-drain />");
    write(
        &root.join("client/components/test/drain/tac.html"),
        "<main><h1>Drain</h1></main>",
    );
    let started = root.join("slow.started");
    write(
        &root.join("server/controllers/SlowController.js"),
        &format!(
            r"
@RequestRoute('/slow')
@Public
export class SlowController {{
  @GetRoute
  async get() {{
    const fs = await import('node:fs')
    fs.writeFileSync({started:?}, 'started')
    await new Promise((resolve) => setTimeout(resolve, 1500))
    return {{ finished: true }}
  }}
}}
"
        ),
    );
}

async fn read_response(mut stream: TcpStream) -> String {
    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(30), stream.read_to_end(&mut response))
        .await
        .expect("response within the deadline")
        .expect("read");
    String::from_utf8_lossy(&response).into_owned()
}

#[tokio::test]
async fn shutdown_lets_an_in_flight_request_finish_before_the_server_stops() {
    let project = tempfile::tempdir().expect("project");
    write_slow_project(project.path());
    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = shutdown.await;
            })
            .await
    });

    let mut client = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    client
        .write_all(b"GET /slow HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .await
        .expect("request");
    let started = project.path().join("slow.started");
    tokio::time::timeout(Duration::from_secs(30), async {
        while !started.exists() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("handler entered user code");

    stop.send(()).expect("shutdown signal");
    let response = read_response(client).await;
    assert!(response.contains("200 OK"), "{response}");
    assert!(response.contains("\"finished\":true"), "{response}");
    tokio::time::timeout(Duration::from_secs(10), running)
        .await
        .expect("bounded server shutdown")
        .expect("server task")
        .expect("clean shutdown");
}
