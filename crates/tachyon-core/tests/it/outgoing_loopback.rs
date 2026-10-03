#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

struct LoopbackProbe;

impl LoopbackProbe {
    const DATA_SOURCE: &str =
        r#"{"datasource":{"driver":"sqlite","url":"sqlite::memory:"},"cohesion":{"check":false}}"#;
    const REACHED: &str = "reached-the-private-loopback-service";
    const JAVASCRIPT: &str = r"
@RequestRoute('/probe')
@Public
export class ProbeController {
  @GetRoute
  async get(@FromQuery('target') target) {
    try {
      const answer = await fetch(target)
      return `${answer.status} ${await answer.text()}`
    } catch (error) {
      return `failed ${error} ${error.cause?.message ?? ''}`
    }
  }
}
";
    const GO: &str = r#"import (
	"io"
	"net/http"
	"strconv"
)

// @RequestRoute("/probe")
// @Public
type ProbeController struct {
}

// @GetRoute
func (controller *ProbeController) GetProbe(
// @FromQuery("target")
 target string) string {
	answer, err := http.Get(target)
	if err != nil {
		return "failed " + err.Error()
	}
	defer answer.Body.Close()
	body, _ := io.ReadAll(answer.Body)
	return strconv.Itoa(answer.StatusCode) + " " + string(body)
}
"#;

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("file");
    }

    fn available(program: &str) -> bool {
        std::process::Command::new(program)
            .arg("version")
            .output()
            .is_ok_and(|output| output.status.success())
            || std::process::Command::new(program)
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success())
    }

    async fn private_service() -> (u16, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("service");
        let port = listener.local_addr().expect("address").port();
        let connections = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&connections);
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                counted.fetch_add(1, Ordering::SeqCst);
                let mut request = vec![0_u8; 4096];
                let _read = stream.read(&mut request).await;
                let answer = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    Self::REACHED.len(),
                    Self::REACHED
                );
                let _written = stream.write_all(answer.as_bytes()).await;
            }
        });
        (port, connections)
    }

    async fn ask(port: u16, path: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        let raw = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
        stream.write_all(raw.as_bytes()).await.expect("write");
        let mut answer = Vec::new();
        tokio::time::timeout(Duration::from_mins(2), stream.read_to_end(&mut answer))
            .await
            .expect("answer within the deadline")
            .expect("read");
        String::from_utf8_lossy(&answer).into_owned()
    }

    async fn answer(file: &str, source: &str, service: u16) -> String {
        let project = tempfile::tempdir().expect("project");
        Self::write(
            &project.path().join("server/application.json"),
            Self::DATA_SOURCE,
        );
        Self::write(
            &project.path().join("server/controllers").join(file),
            source,
        );
        let options = DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        };
        let server = DevServer::bind(project.path(), &options)
            .await
            .expect("bind");
        let port = server.address().port();
        let (stop, wait) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(server.run_until(async {
            let _stopped = wait.await;
        }));
        let target = format!("http://127.0.0.1:{service}/");
        let answer = Self::ask(port, &format!("/probe?target={target}")).await;
        let _stopped = stop.send(());
        answer
    }

    fn assert_guarded(language: &str, answer: &str, connections: &AtomicUsize) {
        assert!(
            !answer.contains(Self::REACHED),
            "{language} reached an unlisted loopback port without the outgoing guard: {answer}"
        );
        assert_eq!(
            connections.load(Ordering::SeqCst),
            0,
            "{language}: {answer}"
        );
        assert!(answer.contains("200 OK"), "{language}: {answer}");
    }
}

#[tokio::test]
async fn a_handler_with_a_resource_endpoint_cannot_skip_the_guard_to_reach_loopback() {
    let (service, connections) = LoopbackProbe::private_service().await;
    let answer =
        LoopbackProbe::answer("ProbeController.js", LoopbackProbe::JAVASCRIPT, service).await;
    LoopbackProbe::assert_guarded("JavaScript", &answer, &connections);
}

#[tokio::test]
async fn a_go_handler_cannot_skip_the_guard_to_reach_loopback() {
    if !LoopbackProbe::available("go") {
        return;
    }
    let (service, connections) = LoopbackProbe::private_service().await;
    let answer = LoopbackProbe::answer("ProbeController.go", LoopbackProbe::GO, service).await;
    LoopbackProbe::assert_guarded("Go", &answer, &connections);
    assert!(answer.contains("403 Tachyon refused"), "Go: {answer}");
}
