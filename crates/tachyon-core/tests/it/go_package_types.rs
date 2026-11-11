#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

struct GoPackageTypes;

impl GoPackageTypes {
    const FILES: [(&'static str, &'static str); 5] = [
        (
            "server/packages/go.mod",
            "module example.com/shop/packages\n\ngo 1.27\n",
        ),
        (
            "server/packages/core/core.go",
            "package core\n\ntype Order struct {\n\tName string `json:\"name\"`\n}\n\ntype OrderMissing struct{}\n\nfunc (failure *OrderMissing) Error() string { return \"order missing\" }\n",
        ),
        (
            "server/models/Problem.go",
            "type Problem struct {\n\tCode string `json:\"code\"`\n}\n",
        ),
        (
            "server/controllers/OrdersController.go",
            "import \"example.com/shop/packages/core\"\n\n// @RequestRoute(\"/orders\")\n// @Public\ntype OrdersController struct{}\n\n// @GetRoute\nfunc (controller *OrdersController) GetOrder() (Problem, error) {\n\treturn Problem{}, &core.OrderMissing{}\n}\n\n// @PostRoute\nfunc (controller *OrdersController) PostOrder(\n\t// @FromBody\n\torder core.Order,\n) Problem {\n\treturn Problem{Code: order.Name}\n}\n",
        ),
        (
            "server/controllers/OrdersErrorHandler.go",
            "import alias \"example.com/shop/packages/core\"\n\n// @ErrorHandler\ntype OrdersErrorHandler struct{}\n\n// @HandlesError\n// @ResponseStatus(404)\nfunc (handler *OrdersErrorHandler) Missing(failure *alias.OrderMissing) Problem {\n\treturn Problem{Code: \"missing\"}\n}\n",
        ),
    ];

    fn usable() -> bool {
        let found = crate::support::program_probe::ProgramProbe::answers("go", "version");
        crate::support::toolchain::Toolchain::checked("go", found)
    }

    fn write(root: &Path) {
        for (relative, source) in Self::FILES {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("folders");
            fs::write(path, source).expect("source");
        }
    }

    async fn send(port: u16, request: String) -> String {
        let mut socket = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        socket.write_all(request.as_bytes()).await.expect("request");
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_mins(2), socket.read_to_end(&mut response))
            .await
            .expect("response deadline")
            .expect("response");
        String::from_utf8(response).expect("UTF-8 response")
    }

    fn get(path: &str) -> String {
        format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
    }

    fn post(path: &str, body: &str) -> String {
        format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    async fn answers(root: &Path, requests: Vec<String>) -> Vec<String> {
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
        let mut answers = Vec::new();
        for request in requests {
            answers.push(Self::send(port, request).await);
        }
        stop.send(()).expect("stop");
        running.await.expect("task").expect("shutdown");
        answers
    }
}

#[tokio::test]
async fn go_body_and_error_handler_types_may_come_from_a_native_package() {
    if !GoPackageTypes::usable() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    GoPackageTypes::write(root.path());
    let answers = GoPackageTypes::answers(
        root.path(),
        vec![
            GoPackageTypes::get("/orders"),
            GoPackageTypes::post("/orders", r#"{"name":"pen"}"#),
        ],
    )
    .await;
    assert!(answers[0].starts_with("HTTP/1.1 404 "), "{}", answers[0]);
    assert!(answers[0].contains(r#""code":"missing""#), "{}", answers[0]);
    assert!(answers[1].starts_with("HTTP/1.1 200 "), "{}", answers[1]);
    assert!(answers[1].contains(r#""code":"pen""#), "{}", answers[1]);
}
