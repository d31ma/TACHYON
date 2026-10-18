#![allow(clippy::expect_used)]

use crate::support::json_http::JsonHttp;
use std::path::Path;
use std::process::{Command, Stdio};

struct RustCheckProject {
    files: &'static [(&'static str, &'static str)],
}

impl RustCheckProject {
    const MODEL: (&'static str, &'static str) = (
        "models/OrderInput",
        "#[Checked]\npub struct OrderInput {\n    #[NotEmpty]\n    name: String,\n    #[Required]\n    #[Min(1)]\n    level: f64,\n}\n",
    );
    const PROBLEM: (&'static str, &'static str) = (
        "models/Problem",
        "pub struct Problem {\n    pub code: String,\n}\n",
    );
    const SERVICE: (&'static str, &'static str) = (
        "services/OrderService",
        "use crate::models::OrderInput;\n\n#[yon::service]\nstruct OrderService {}\nimpl OrderService {\n    fn new() -> Self { Self {} }\n    fn accept(&self, #[Check(OrderInput)] order: BehaviorValue) -> BehaviorResult<BehaviorValue> { Ok(order) }\n}\n",
    );
    const CONTROLLER: (&'static str, &'static str) = (
        "controllers/OrdersController",
        r#"use std::sync::Arc;

#[RequestRoute("/orders")]
#[Public]
struct OrdersController {
    #[Needs]
    orders: Arc<OrderService>,
}
impl OrdersController {
    #[PostRoute]
    fn post_orders(&self, #[FromBody] order: Json) -> Result<Json, BehaviorError> {
        if order.get("fail") == &Json::Bool(true) {
            return Err(BehaviorError::new("adapter", "PRIVATE_ERROR_SENTINEL"));
        }
        self.orders.accept(order.into()).map(Json::from)
    }
}
"#,
    );
    const OTHER_ERROR: (&'static str, &'static str) = (
        "errors/OrderGone",
        "#[derive(Debug)]\npub struct OrderGone;\n\nimpl std::fmt::Display for OrderGone {\n    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n        formatter.write_str(\"gone\")\n    }\n}\n\nimpl std::error::Error for OrderGone {}\n",
    );
    const OTHER_HANDLER: (&'static str, &'static str) = (
        "controllers/OrdersErrorHandler",
        "use crate::models::Problem;\n\n#[ErrorHandler]\nstruct OrdersErrorHandler {}\n\nimpl OrdersErrorHandler {\n    fn new() -> Self {\n        Self {}\n    }\n\n    #[HandlesError]\n    #[ResponseStatus(410)]\n    fn gone(&self, error: &OrderGone) -> Problem {\n        Problem { code: String::from(\"gone\") }\n    }\n}\n",
    );
    const CHECK_HANDLER: (&'static str, &'static str) = (
        "controllers/OrdersErrorHandler",
        "use crate::models::Problem;\n\n#[ErrorHandler]\nstruct OrdersErrorHandler {}\n\nimpl OrdersErrorHandler {\n    fn new() -> Self {\n        Self {}\n    }\n\n    #[HandlesError]\n    #[ResponseStatus(400)]\n    fn invalid(&self, error: &BehaviorError) -> Problem {\n        Problem { code: error.path.clone().unwrap_or_default() }\n    }\n}\n",
    );
    const UNHANDLED: Self = Self {
        files: &[
            Self::MODEL,
            Self::PROBLEM,
            Self::SERVICE,
            Self::CONTROLLER,
            Self::OTHER_ERROR,
            Self::OTHER_HANDLER,
        ],
    };
    const HANDLED: Self = Self {
        files: &[
            Self::MODEL,
            Self::PROBLEM,
            Self::SERVICE,
            Self::CONTROLLER,
            Self::CHECK_HANDLER,
        ],
    };

    fn available() -> bool {
        let found = Command::new("rustc")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if !found {
            assert!(std::env::var_os("CI").is_none(), "missing rustc in CI");
            eprintln!("skipping Rust check rejections: missing rustc");
        }
        found
    }

    async fn answers(&self, bodies: &[&str]) -> Vec<String> {
        let root = tempfile::tempdir().expect("project");
        self.write(root.path());
        JsonHttp::post_all(root.path(), "/orders", bodies).await
    }

    fn write(&self, root: &Path) {
        for (name, source) in self.files {
            let destination = root.join(format!("server/{name}.rs"));
            std::fs::create_dir_all(destination.parent().expect("parent")).expect("folder");
            std::fs::write(destination, source).expect("source");
        }
    }

    fn body(response: &str) -> &str {
        response.split_once("\r\n\r\n").map_or("", |(_, body)| body)
    }
}

#[tokio::test]
async fn an_unhandled_rust_check_rejection_answers_422_and_other_errors_stay_private() {
    if !RustCheckProject::available() {
        return;
    }
    let answers = RustCheckProject::UNHANDLED
        .answers(&[
            r#"{"name":"ok","level":1}"#,
            r#"{"name":" ","level":0}"#,
            r#"{"fail":true}"#,
        ])
        .await;
    assert!(answers[0].starts_with("HTTP/1.1 200 OK"), "{}", answers[0]);
    assert!(
        answers[1].starts_with("HTTP/1.1 422 Unprocessable Entity"),
        "{}",
        answers[1]
    );
    assert!(
        RustCheckProject::body(&answers[1]).contains(
            r#""errors":[{"field":"level","rule":"Min"},{"field":"name","rule":"NotEmpty"}]"#
        ),
        "{}",
        answers[1]
    );
    assert!(
        answers[2].starts_with("HTTP/1.1 500 Internal Server Error"),
        "{}",
        answers[2]
    );
    assert!(
        !answers[2].contains("PRIVATE_ERROR_SENTINEL"),
        "{}",
        answers[2]
    );
}

#[tokio::test]
async fn a_rust_error_handler_still_catches_a_check_rejection_by_its_first_path() {
    if !RustCheckProject::available() {
        return;
    }
    let answers = RustCheckProject::HANDLED
        .answers(&[r#"{"name":"ok"}"#])
        .await;
    assert!(
        answers[0].starts_with("HTTP/1.1 400 Bad Request"),
        "{}",
        answers[0]
    );
    assert_eq!(
        RustCheckProject::body(&answers[0]),
        r#"{"code":"level.notNull"}"#,
        "{}",
        answers[0]
    );
}
