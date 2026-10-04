#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;

struct EchoProject;

impl EchoProject {
    const JAVASCRIPT: &str = r"
@RequestRoute('/echo')
@Public
export class EchoController {
  @GetRoute
  get(@FromHeader('x-forwarded-for') forwardedFor, @FromHeader('x-real-ip') realIp, @FromHeader('forwarded') forwarded) {
    return { forwardedFor, realIp, forwarded }
  }
}
";
    const GO: &str = "import \"net/http\"\n\n// @RequestRoute(\"/peer\")\n// @Public\ntype PeerController struct {\n}\n\n// @GetRoute\nfunc (controller *PeerController) GetPeer(request *http.Request) map[string]string {\n\treturn map[string]string{\"remote\": request.RemoteAddr, \"forwardedFor\": request.Header.Get(\"X-Forwarded-For\")}\n}\n";
    const SPOOFED: &str =
        "X-Forwarded-For: 6.6.6.6\r\nX-Real-IP: 6.6.6.6\r\nForwarded: for=6.6.6.6\r\n";

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("file");
    }

    fn create(controller: &str, source: &str, application: &str) -> tempfile::TempDir {
        let project = tempfile::tempdir().expect("project");
        Self::write(project.path(), controller, source);
        Self::write(project.path(), "server/application.json", application);
        project
    }

    async fn ask(project: &Path, path: &str, headers: &str) -> String {
        let options = DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        };
        let server = DevServer::bind(project, &options).await.expect("bind");
        let port = server.address().port();
        let (stop, wait) = oneshot::channel::<()>();
        tokio::spawn(server.run_until(async {
            let _stopped = wait.await;
        }));
        let raw =
            format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{headers}Connection: close\r\n\r\n");
        let mut stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        stream.write_all(raw.as_bytes()).await.expect("write");
        let mut answer = Vec::new();
        tokio::time::timeout(Duration::from_mins(2), stream.read_to_end(&mut answer))
            .await
            .expect("answer within the deadline")
            .expect("read");
        let _stopped = stop.send(());
        String::from_utf8_lossy(&answer).into_owned()
    }

    fn go_usable() -> bool {
        std::process::Command::new("go")
            .arg("version")
            .output()
            .is_ok_and(|output| output.status.success())
    }
}

#[tokio::test]
async fn a_client_cannot_spoof_its_address_to_a_yon_handler() {
    let project = EchoProject::create(
        "server/controllers/EchoController.js",
        EchoProject::JAVASCRIPT,
        "{}",
    );

    let answer = EchoProject::ask(project.path(), "/echo", EchoProject::SPOOFED).await;

    assert!(answer.contains("\"realIp\":\"127.0.0.1\""), "{answer}");
    assert!(
        answer.contains("\"forwardedFor\":\"127.0.0.1\""),
        "{answer}"
    );
    assert!(
        answer.contains("\"forwarded\":\"for=127.0.0.1;proto=http\""),
        "{answer}"
    );
    assert!(!answer.contains("6.6.6.6"), "{answer}");
}

#[tokio::test]
async fn a_configured_trusted_proxy_passes_the_client_address_on() {
    let project = EchoProject::create(
        "server/controllers/EchoController.js",
        EchoProject::JAVASCRIPT,
        r#"{"server":{"proxies":["127.0.0.1"]}}"#,
    );

    let answer = EchoProject::ask(
        project.path(),
        "/echo",
        "X-Forwarded-For: 6.6.6.6, 198.51.100.4\r\n",
    )
    .await;

    assert!(answer.contains("\"realIp\":\"198.51.100.4\""), "{answer}");
    assert!(
        answer.contains("\"forwardedFor\":\"6.6.6.6, 198.51.100.4, 127.0.0.1\""),
        "{answer}"
    );
}

#[tokio::test]
async fn a_go_handler_reads_the_real_peer_from_its_native_request() {
    if !EchoProject::go_usable() {
        return;
    }
    let project = EchoProject::create(
        "server/controllers/PeerController.go",
        EchoProject::GO,
        "{}",
    );

    let answer = EchoProject::ask(project.path(), "/peer", EchoProject::SPOOFED).await;

    assert!(answer.contains("\"remote\":\"127.0.0.1:0\""), "{answer}");
    assert!(
        answer.contains("\"forwardedFor\":\"127.0.0.1\""),
        "{answer}"
    );
    assert!(!answer.contains("6.6.6.6"), "{answer}");
}
