#![allow(clippy::expect_used)]

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_contracts::{HandlerBody, HandlerBodyEncoding, HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct Worker {
    extension: &'static str,
    program: &'static str,
    source: &'static str,
}

impl Worker {
    const SCHEDULE: &'static str = r#"{"fixedDelaySeconds":300,"initialDelaySeconds":30}"#;

    const JAVASCRIPT: Self = Self {
        extension: "js",
        program: "node",
        source: "@Schedule(__SCHEDULE__)\nexport class Handler {\n  static POST() {\n  }\n}\n",
    };

    const TYPESCRIPT: Self = Self {
        extension: "ts",
        program: "bun",
        source: "@Schedule(__SCHEDULE__)\nexport class Handler {\n  static POST(): void {\n  }\n}\n",
    };

    const PYTHON: Self = Self {
        extension: "py",
        program: "python3",
        source: "@schedule(__SCHEDULE__)\nclass Handler:\n    @staticmethod\n    def POST():\n        pass\n",
    };

    const RUBY: Self = Self {
        extension: "rb",
        program: "ruby",
        source: "#[Schedule(__SCHEDULE__)]\nclass Handler\n  def self.POST\n  end\nend\n",
    };

    const PHP: Self = Self {
        extension: "php",
        program: "php-cgi",
        source: "<?php\n#[Schedule(__SCHEDULE__)]\nclass Handler {\n    public static function POST(): void {\n    }\n}\n",
    };

    const JAVA: Self = Self {
        extension: "java",
        program: "javac",
        source: "@Schedule(__SCHEDULE__)\nclass Handler {\n    static void POST() {\n    }\n}\n",
    };

    const KOTLIN: Self = Self {
        extension: "kt",
        program: "kotlinc",
        source: "@Schedule(__SCHEDULE__)\nobject Handler {\n    fun POST() {\n    }\n}\n",
    };

    const CSHARP: Self = Self {
        extension: "cs",
        program: "dotnet",
        source: "[Schedule(__SCHEDULE__)]\npublic static class Handler {\n    public static void POST() {\n    }\n}\n",
    };

    const RUST: Self = Self {
        extension: "rs",
        program: "cargo",
        source: "#[Schedule(__SCHEDULE__)]\nstruct Handler {}\nimpl Handler {\n    fn POST() {\n    }\n}\n",
    };

    const DART: Self = Self {
        extension: "dart",
        program: "dart",
        source: "@Schedule(__SCHEDULE__)\nclass Handler {\n  void POST() {\n  }\n}\n",
    };

    const GO: Self = Self {
        extension: "go",
        program: "go",
        source: "// @Schedule(__SCHEDULE__)\ntype Handler struct{}\n\nfunc (Handler) POST() {\n}\n",
    };

    async fn assert_runs(&self) {
        if Tools::skipped(self.program, self.extension) {
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let entry = format!("server/workers/reconcile.{}", self.extension);
        Tools::write(
            root.path(),
            &entry,
            &self.source.replace("__SCHEDULE__", Self::SCHEDULE),
        );
        let request = HandlerRequest::route("reconcile", "/", HttpMethod::Post);
        let (status, body) = Tools::invoke(root.path(), &entry, self.extension, &request).await;
        assert!(
            (200..300).contains(&status),
            "{}: {status} {body}",
            self.extension
        );
    }
}

struct Tools;

impl Tools {
    fn available(program: &str) -> bool {
        let flag = match program {
            "kotlinc" => "-version",
            "go" => "version",
            _ => "--version",
        };
        Command::new(program)
            .arg(flag)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn skipped(program: &str, extension: &str) -> bool {
        if Self::available(program) {
            return false;
        }
        assert!(
            std::env::var_os("CI").is_none(),
            "{program} is required in CI"
        );
        eprintln!("skipping {extension}: {program} is unavailable");
        true
    }

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
        std::fs::write(path, source).expect("source");
    }

    async fn invoke(
        root: &Path,
        entry: &str,
        extension: &str,
        request: &HandlerRequest,
    ) -> (u16, String) {
        let source = HandlerSource::discover(root, entry)
            .unwrap_or_else(|failure| panic!("{extension}: {failure}"));
        let mut options = HandlerSupervisorOptions {
            default_timeout: Duration::from_mins(2),
            ..HandlerSupervisorOptions::default()
        };
        if extension == "ts" {
            options.runtimes.javascript = "bun".into();
        }
        let supervisor = HandlerSupervisor::new(options).expect("supervisor");
        let response = supervisor
            .invoke(&source, request, &HandlerCancellation::default())
            .await
            .expect("invocation");
        assert!(
            response.error.is_none(),
            "{extension}: {:?}",
            response.error
        );
        let body = response.body.map(|body| body.data).unwrap_or_default();
        (response.status, body)
    }
}

#[tokio::test]
async fn a_javascript_worker_needs_no_request_parameter() {
    Worker::JAVASCRIPT.assert_runs().await;
}

#[tokio::test]
async fn a_typescript_worker_needs_no_request_parameter() {
    Worker::TYPESCRIPT.assert_runs().await;
}

#[tokio::test]
async fn a_python_worker_needs_no_request_parameter() {
    Worker::PYTHON.assert_runs().await;
}

#[tokio::test]
async fn a_ruby_worker_needs_no_request_parameter() {
    Worker::RUBY.assert_runs().await;
}

#[tokio::test]
async fn a_php_worker_needs_no_request_parameter() {
    Worker::PHP.assert_runs().await;
}

#[tokio::test]
async fn a_java_worker_needs_no_request_parameter() {
    Worker::JAVA.assert_runs().await;
}

#[tokio::test]
async fn a_kotlin_worker_needs_no_request_parameter() {
    Worker::KOTLIN.assert_runs().await;
}

#[tokio::test]
async fn a_csharp_worker_needs_no_request_parameter() {
    Worker::CSHARP.assert_runs().await;
}

#[tokio::test]
async fn a_rust_worker_needs_no_request_parameter() {
    Worker::RUST.assert_runs().await;
}

#[tokio::test]
async fn a_dart_worker_needs_no_request_parameter() {
    Worker::DART.assert_runs().await;
}

#[tokio::test]
async fn a_go_worker_needs_no_request_parameter() {
    Worker::GO.assert_runs().await;
}

#[tokio::test]
async fn a_csharp_controller_reads_a_behavior_json_body() {
    if Tools::skipped("dotnet", "cs") {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    Tools::write(
        root.path(),
        "server/services/OrderService.cs",
        "[Service]\nclass OrderService {\n    public BehaviorJSON accept([Check(typeof(OrderInput))] BehaviorJSON order) { return order; }\n}\n",
    );
    Tools::write(
        root.path(),
        "server/helpers/OrderInput.cs",
        "[Checked]\nclass OrderInput {\n    [NotEmpty]\n    public BehaviorJSON name;\n}\n",
    );
    Tools::write(
        root.path(),
        "server/controllers/OrdersController.cs",
        "[RequestRoute(\"/orders\")]\n[Public]\nclass OrdersController {\n    [Needs]\n    readonly OrderService _orders;\n    [PostRoute]\n    public BehaviorJSON PostOrders([FromBody] BehaviorJSON order) => _orders.accept(order);\n}\n",
    );
    let mut request = HandlerRequest::route("orders", "/orders", HttpMethod::Post);
    request.arguments = Some(vec![serde_json::json!({"name": "anvil", "level": 4})]);
    request.body = Some(HandlerBody {
        encoding: HandlerBodyEncoding::Utf8,
        data: String::from(r#"{"name":"anvil","level":4}"#),
    });
    let (status, body) = Tools::invoke(
        root.path(),
        "server/controllers/OrdersController.cs",
        "cs",
        &request,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("JSON answer");
    assert_eq!(value, serde_json::json!({"name": "anvil", "level": 4}));
}
