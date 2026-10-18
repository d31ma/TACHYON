#![allow(clippy::expect_used)]

use std::path::Path;
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

pub struct JsonHttp;

impl JsonHttp {
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

    pub async fn post_all(root: &Path, path: &str, bodies: &[&str]) -> Vec<String> {
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
