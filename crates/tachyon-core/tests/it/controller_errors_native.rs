#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

struct Http;

impl Http {
    async fn get(port: u16, path: &str) -> String {
        let mut socket = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        socket.write_all(request.as_bytes()).await.expect("request");
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_mins(2), socket.read_to_end(&mut response))
            .await
            .expect("response deadline")
            .expect("response");
        String::from_utf8(response).expect("UTF-8 response")
    }

    fn json_body(response: &str) -> serde_json::Value {
        let body = response.split_once("\r\n\r\n").expect("body").1;
        serde_json::from_str(body).expect("JSON body")
    }
}

struct Server;

impl Server {
    async fn responses(root: &Path, paths: &[&str]) -> Vec<String> {
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
        for path in paths {
            responses.push(Http::get(port, path).await);
        }
        stop.send(()).expect("stop");
        running.await.expect("task").expect("shutdown");
        responses
    }
}

struct Answer {
    path: &'static str,
    status: u16,
    code: &'static str,
}

impl Answer {
    const fn handled(path: &'static str, status: u16, code: &'static str) -> Self {
        Self { path, status, code }
    }

    const fn safe_failure(path: &'static str) -> Self {
        Self {
            path,
            status: 500,
            code: "",
        }
    }

    fn check(&self, extension: &str, response: &str) {
        let path = self.path;
        let status_line = format!("HTTP/1.1 {}", self.status);
        assert!(
            response.starts_with(&status_line),
            "{extension} {path}: {response}"
        );
        assert!(
            !response.contains("PRIVATE_ERROR_SENTINEL"),
            "{extension} {path}: {response}"
        );
        if !self.code.is_empty() {
            let expected = serde_json::json!({ "code": self.code });
            assert_eq!(Http::json_body(response), expected, "{extension} {path}");
        }
    }
}

struct Project {
    extension: &'static str,
    program: &'static str,
    version_flag: &'static str,
    files: &'static [(&'static str, &'static str)],
    answers: &'static [Answer],
    published: &'static [(&'static str, u16)],
}

impl Project {
    fn available(&self) -> bool {
        let status = Command::new(self.program)
            .arg(self.version_flag)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if status.is_ok_and(|status| status.success()) {
            return true;
        }
        assert!(
            std::env::var_os("CI").is_none(),
            "missing {} in CI",
            self.program
        );
        eprintln!(
            "skipping {} error handlers: missing {}",
            self.extension, self.program
        );
        false
    }

    fn write(&self, root: &Path) {
        for (name, source) in self.files {
            let destination = root.join(format!("server/{name}.{}", self.extension));
            fs::create_dir_all(destination.parent().expect("parent")).expect("directory");
            fs::write(destination, source).expect("source");
        }
    }

    async fn verify(&self) {
        if !self.available() {
            return;
        }
        let root = tempfile::tempdir().expect("project");
        self.write(root.path());
        let mut paths = vec!["/api.json"];
        for answer in self.answers {
            paths.push(answer.path);
        }
        let responses = Server::responses(root.path(), &paths).await;
        self.check_schema(&responses[0]);
        for (answer, response) in self.answers.iter().zip(&responses[1..]) {
            answer.check(self.extension, response);
        }
    }

    fn check_schema(&self, response: &str) {
        let schema = Http::json_body(response);
        let mut published = Vec::new();
        for route in schema["routes"].as_array().expect("routes") {
            let Some(errors) = route["methods"]["GET"]["errors"].as_array() else {
                continue;
            };
            for error in errors {
                let name = error["type"].as_str().expect("error type");
                let name = name.trim_start_matches(['&', '*']).to_owned();
                let status = error["status"].as_u64().expect("error status");
                published.push((name, status));
            }
        }
        for (name, status) in self.published {
            let found = published.iter().any(|(published_name, published_status)| {
                published_name.ends_with(name) && *published_status == u64::from(*status)
            });
            assert!(found, "{}: {name} {status} not in {schema}", self.extension);
        }
    }
}

struct GoProject;

impl GoProject {
    const ERRORS: [(&'static str, &'static str); 4] = [
        (
            "errors/OrderNotFound",
            "type OrderNotFound struct{}\n\nfunc (failure *OrderNotFound) Error() string { return \"PRIVATE_ERROR_SENTINEL\" }\n",
        ),
        (
            "errors/OrderGone",
            "type OrderGone struct{}\n\nfunc (failure *OrderGone) Error() string { return \"PRIVATE_ERROR_SENTINEL\" }\n\nfunc (failure *OrderGone) Unwrap() error { return &OrderNotFound{} }\n",
        ),
        (
            "errors/OrderBroken",
            "type OrderBroken struct{}\n\nfunc (failure *OrderBroken) Error() string { return \"PRIVATE_ERROR_SENTINEL\" }\n",
        ),
        (
            "models/Problem",
            "type Problem struct {\n\tCode string `json:\"code\"`\n}\n",
        ),
    ];

    const SERVICE: (&'static str, &'static str) = (
        "services/OrdersService",
        r#"import "fmt"

// @Service()
type OrdersService struct{}

func (service *OrdersService) Find() (Problem, error) {
	return Problem{}, fmt.Errorf("PRIVATE_ERROR_SENTINEL: %w", &OrderNotFound{})
}
"#,
    );

    const CONTROLLERS: [(&'static str, &'static str); 2] = [
        (
            "controllers/OrdersController",
            r#"import "fmt"

// @RequestRoute("/orders")
// @Public
// @NeedsAll()
type OrdersController struct {
	orders *OrdersService
}

func (controller *OrdersController) privateMessage() string {
	return "PRIVATE_ERROR_SENTINEL"
}

// @GetRoute
func (controller *OrdersController) GetOrder() (Problem, error) {
	return Problem{}, &OrderNotFound{}
}

// @GetRoute("/gone")
func (controller *OrdersController) GetGone() (Problem, error) {
	return Problem{}, fmt.Errorf("PRIVATE_ERROR_SENTINEL: %w", &OrderGone{})
}

// @GetRoute("/service")
func (controller *OrdersController) GetService() (Problem, error) {
	return controller.orders.Find()
}

// @GetRoute("/unknown")
func (controller *OrdersController) GetUnknown() (Problem, error) {
	return Problem{}, fmt.Errorf("%s", controller.privateMessage())
}

// @GetRoute("/broken")
func (controller *OrdersController) GetBroken() (Problem, error) {
	return Problem{}, &OrderBroken{}
}

// @GetRoute("/plain")
func (controller *OrdersController) GetPlain() Problem {
	return Problem{Code: "plain"}
}

// @GetRoute("/panic")
func (controller *OrdersController) GetPanic() Problem {
	panic(controller.privateMessage())
}
"#,
        ),
        (
            "controllers/StatusController",
            r#"// @RequestRoute("/status")
// @Public
type StatusController struct{}

// @GetRoute
func (controller *StatusController) GetStatus() (Problem, error) {
	return Problem{}, &OrderNotFound{}
}
"#,
        ),
    ];

    const HANDLER: (&'static str, &'static str) = (
        "controllers/OrdersErrorHandler",
        r#"import (
	"fmt"
	"net/http"
)

// @ErrorHandler
type OrdersErrorHandler struct{}

// @HandlesError
// @ResponseStatus(404)
func (handler *OrdersErrorHandler) Missing(failure *OrderNotFound) Problem {
	return Problem{Code: "missing"}
}

// @HandlesError
// @ResponseStatus(410)
func (handler *OrdersErrorHandler) Gone(failure *OrderGone) Problem {
	return Problem{Code: "gone"}
}

// @HandlesError
// @ResponseStatus(503)
func (handler *OrdersErrorHandler) Broken(failure *OrderBroken, request *http.Request) (Problem, error) {
	return Problem{}, fmt.Errorf("PRIVATE_ERROR_SENTINEL")
}
"#,
    );

    const CATCH_ALL_FIRST_HANDLER: (&'static str, &'static str) = (
        "controllers/OrdersErrorHandler",
        r#"// @ErrorHandler
type OrdersErrorHandler struct{}

// @HandlesError
// @ResponseStatus(400)
func (handler *OrdersErrorHandler) Other(failure error) Problem {
	return Problem{Code: "other"}
}

// @HandlesError
// @ResponseStatus(410)
func (handler *OrdersErrorHandler) Gone(failure *OrderGone) Problem {
	return Problem{Code: "gone"}
}

// @HandlesError
// @ResponseStatus(404)
func (handler *OrdersErrorHandler) Missing(failure *OrderNotFound) Problem {
	return Problem{Code: "missing"}
}
"#,
    );

    const FILES: [(&'static str, &'static str); 8] = [
        Self::ERRORS[0],
        Self::ERRORS[1],
        Self::ERRORS[2],
        Self::ERRORS[3],
        Self::SERVICE,
        Self::CONTROLLERS[0],
        Self::CONTROLLERS[1],
        Self::HANDLER,
    ];

    const CATCH_ALL_FIRST_FILES: [(&'static str, &'static str); 8] = [
        Self::ERRORS[0],
        Self::ERRORS[1],
        Self::ERRORS[2],
        Self::ERRORS[3],
        Self::SERVICE,
        Self::CONTROLLERS[0],
        Self::CONTROLLERS[1],
        Self::CATCH_ALL_FIRST_HANDLER,
    ];

    const PROJECT: Project = Project {
        extension: "go",
        program: "go",
        version_flag: "version",
        files: &Self::FILES,
        answers: &[
            Answer::handled("/orders", 404, "missing"),
            Answer::handled("/orders/gone", 410, "gone"),
            Answer::handled("/orders/service", 404, "missing"),
            Answer::handled("/status", 404, "missing"),
            Answer::handled("/orders/plain", 200, "plain"),
            Answer::safe_failure("/orders/unknown"),
            Answer::safe_failure("/orders/broken"),
            Answer::safe_failure("/orders/panic"),
        ],
        published: &[("OrderNotFound", 404), ("OrderGone", 410)],
    };

    const CATCH_ALL_FIRST_PROJECT: Project = Project {
        extension: "go",
        program: "go",
        version_flag: "version",
        files: &Self::CATCH_ALL_FIRST_FILES,
        answers: &[
            Answer::handled("/orders", 404, "missing"),
            Answer::handled("/orders/gone", 410, "gone"),
            Answer::handled("/orders/service", 404, "missing"),
            Answer::handled("/status", 404, "missing"),
            Answer::handled("/orders/unknown", 400, "other"),
            Answer::handled("/orders/broken", 400, "other"),
        ],
        published: &[("OrderNotFound", 404), ("OrderGone", 410), ("error", 400)],
    };
}

struct RustProject;

impl RustProject {
    const FILES: [(&'static str, &'static str); 8] = [
        (
            "errors/OrderNotFound",
            r#"#[derive(Debug)]
pub struct OrderNotFound;

impl std::fmt::Display for OrderNotFound {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PRIVATE_ERROR_SENTINEL")
    }
}

impl std::error::Error for OrderNotFound {}
"#,
        ),
        (
            "errors/OrderGone",
            r#"#[derive(Debug)]
pub struct OrderGone;

impl std::fmt::Display for OrderGone {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PRIVATE_ERROR_SENTINEL")
    }
}

impl std::error::Error for OrderGone {}
"#,
        ),
        (
            "errors/OrderBroken",
            r#"#[derive(Debug)]
pub struct OrderBroken;

impl std::fmt::Display for OrderBroken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PRIVATE_ERROR_SENTINEL")
    }
}

impl std::error::Error for OrderBroken {}
"#,
        ),
        (
            "models/Problem",
            "pub struct Problem {\n    pub code: String,\n}\n",
        ),
        (
            "services/OrdersService",
            r"use crate::models::Problem;

#[Service()]
struct OrdersService {}

impl OrdersService {
    fn new() -> Self {
        Self {}
    }

    fn find(&self) -> Result<Problem, OrderNotFound> {
        Err(OrderNotFound)
    }
}
",
        ),
        (
            "controllers/OrdersController",
            r#"use crate::models::Problem;

#[RequestRoute("/orders")]
#[Public]
#[NeedsAll()]
struct OrdersController {
    orders: std::sync::Arc<OrdersService>,
}

impl OrdersController {
    const PRIVATE_MESSAGE: &str = "PRIVATE_ERROR_SENTINEL";

    #[GetRoute]
    fn get_order(&self) -> Result<Problem, OrderNotFound> {
        Err(OrderNotFound)
    }

    #[GetRoute("/gone")]
    fn get_gone(&self) -> Result<Problem, Box<dyn std::error::Error>> {
        Err(Box::new(OrderGone))
    }

    #[GetRoute("/service")]
    fn get_service(&self) -> Result<Problem, OrderNotFound> {
        let problem = self.orders.find()?;
        Ok(problem)
    }

    #[GetRoute("/unknown")]
    fn get_unknown(&self) -> Result<Problem, std::io::Error> {
        Err(std::io::Error::other(Self::PRIVATE_MESSAGE))
    }

    #[GetRoute("/broken")]
    fn get_broken(&self) -> Result<Problem, OrderBroken> {
        Err(OrderBroken)
    }

    #[GetRoute("/plain")]
    fn get_plain(&self) -> Problem {
        Problem { code: String::from("plain") }
    }

    #[GetRoute("/panic")]
    fn get_panic(&self) -> Problem {
        panic!("{}", Self::PRIVATE_MESSAGE)
    }
}
"#,
        ),
        (
            "controllers/StatusController",
            r#"use crate::models::Problem;

#[RequestRoute("/status")]
#[Public]
struct StatusController {}

impl StatusController {
    fn new() -> Self {
        Self {}
    }

    #[GetRoute]
    fn get_status(&self) -> Result<Problem, Box<dyn std::error::Error>> {
        Err(Box::new(OrderNotFound))
    }
}
"#,
        ),
        (
            "controllers/OrdersErrorHandler",
            r#"use crate::models::Problem;

#[ErrorHandler]
struct OrdersErrorHandler {}

impl OrdersErrorHandler {
    fn new() -> Self {
        Self {}
    }

    #[HandlesError]
    #[ResponseStatus(404)]
    fn missing(&self, error: &OrderNotFound) -> Problem {
        Problem { code: String::from("missing") }
    }

    #[HandlesError]
    #[ResponseStatus(410)]
    fn gone(&self, error: &OrderGone) -> Result<Problem, std::io::Error> {
        Ok(Problem { code: String::from("gone") })
    }

    #[HandlesError]
    #[ResponseStatus(503)]
    fn broken(&self, error: &OrderBroken, request: &http::Request<Vec<u8>>) -> Result<Problem, std::io::Error> {
        Err(std::io::Error::other("PRIVATE_ERROR_SENTINEL"))
    }
}
"#,
        ),
    ];

    const PROJECT: Project = Project {
        extension: "rs",
        program: "rustc",
        version_flag: "--version",
        files: &Self::FILES,
        answers: &[
            Answer::handled("/orders", 404, "missing"),
            Answer::handled("/orders/gone", 410, "gone"),
            Answer::handled("/orders/service", 404, "missing"),
            Answer::handled("/status", 404, "missing"),
            Answer::handled("/orders/plain", 200, "plain"),
            Answer::safe_failure("/orders/unknown"),
            Answer::safe_failure("/orders/broken"),
            Answer::safe_failure("/orders/panic"),
        ],
        published: &[("OrderNotFound", 404), ("OrderGone", 410)],
    };
}

#[tokio::test]
async fn go_error_handlers_answer_every_scenario() {
    GoProject::PROJECT.verify().await;
}

#[tokio::test]
async fn go_concrete_error_handlers_win_over_a_catch_all_declared_first() {
    GoProject::CATCH_ALL_FIRST_PROJECT.verify().await;
}

#[tokio::test]
async fn rust_error_handlers_answer_every_scenario() {
    RustProject::PROJECT.verify().await;
}
