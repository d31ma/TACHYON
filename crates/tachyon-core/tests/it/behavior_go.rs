#![allow(clippy::expect_used)]

use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

use crate::support::authored_behaviors;

fn go_usable() -> bool {
    let available = std::process::Command::new("go")
        .arg("version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if available {
        return true;
    }
    let required = cfg!(unix) && std::env::var_os("CI").is_some();
    assert!(!required, "go is required for the Go behavior tests in CI");
    eprintln!("skipping: go is unavailable");
    false
}

struct GoContract;

impl GoContract {
    const RUNTIME: &str = include_str!("../../src/behavior/polyglot/go_runtime.go");
    const STORAGE: &str = include_str!("../../src/behavior/polyglot/go_storage.go");
    const BROWSER_HTTP: &str = include_str!("../../src/behavior/polyglot/go_browser_http.go");
    const PROGRAM: &str = include_str!("../fixtures/behavior/go_runtime_contract.go");
    const MODULE: &str = "module contract\n\ngo 1.27\n";
    const PACKAGE: &str = "package main\n";

    fn runtime() -> String {
        let mut text = String::from(Self::RUNTIME);
        for part in [Self::STORAGE, Self::BROWSER_HTTP] {
            text.push_str(part.strip_prefix(Self::PACKAGE).unwrap_or(part));
        }
        text
    }
}

#[test]
fn go_behavior_runtime_contracts_execute_with_owned_processes() {
    if !go_usable() {
        return;
    }
    let root = tempfile::tempdir().expect("contract directory");
    std::fs::write(root.path().join("go.mod"), GoContract::MODULE).expect("module file");
    std::fs::write(root.path().join("runtime.go"), GoContract::runtime()).expect("runtime file");
    std::fs::write(root.path().join("contract.go"), GoContract::PROGRAM).expect("contract file");
    let output = std::process::Command::new("go")
        .args(["run", "."])
        .current_dir(root.path())
        .env("GOTOOLCHAIN", "local")
        .env("GOPROXY", "off")
        .env("GOWORK", "off")
        .env("GOFLAGS", "")
        .output()
        .expect("go run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("go behavior runtime contracts passed"),
        "{stdout}"
    );
}

#[tokio::test]
async fn all_go_behavior_annotations_execute() {
    if !go_usable() {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    let relative = "server/controllers/ProductsController.go";
    authored_behaviors::write(project.path(), "go", "server", "go");
    let body = invoke(project.path(), relative).await;
    assert_eq!(body["first"], body["cached"], "{body}");
    assert_eq!(body["first"]["page"]["offset"], 10, "{body}");
    assert_eq!(body["fresh"]["calls"], 2, "{body}");
    assert_eq!(body["remote"], "/products", "{body}");
    assert_eq!(body["saved"]["operation"], "save", "{body}");
    assert_eq!(body["found"]["table"], "products", "{body}");
    assert_eq!(body["deleted"]["operation"], "delete", "{body}");
}

#[tokio::test]
async fn go_behaviors_compose_with_yon_injection() {
    if !go_usable() {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    let root = project.path();
    authored_behaviors::write(root, "go", "server", "go");
    for (relative, source) in GoInjection::FILES {
        std::fs::create_dir_all(root.join(relative).parent().expect("parent")).expect("folders");
        std::fs::write(root.join(relative), source).expect("source");
    }
    let body = invoke(root, "server/controllers/ProductsController.go").await;
    assert_eq!(body["first"], body["cached"], "{body}");
    assert_eq!(body["remote"], "/products", "{body}");
}

#[tokio::test]
async fn a_scheduled_go_worker_runs_without_its_marker() {
    if !go_usable() {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    let relative = "server/workers/beat.go";
    let path = project.path().join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("folders");
    std::fs::write(
        &path,
        "// @Schedule({\"fixedDelaySeconds\":30})\ntype Handler struct{}\n\nfunc (Handler) POST() any {\n\treturn map[string]any{\"beat\": true}\n}\n",
    )
    .expect("worker");
    let source = HandlerSource::discover(project.path(), relative).expect("worker discovery");
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let response = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("go_worker", "/", HttpMethod::Post),
            &HandlerCancellation::default(),
        )
        .await
        .expect("invocation");
    assert!(response.error.is_none(), "{:?}", response.error);
    assert_eq!(response.body.expect("body").data, r#"{"beat":true}"#);
}

pub(super) struct GoInjection;

impl GoInjection {
    pub(super) const FILES: [(&str, &str); 2] = [
        (
            "server/services/CounterService.go",
            "// @yon::Service()\ntype CounterService struct {\n\tvalue int\n}\n\nfunc (counter *CounterService) Value() int { return 42 }\n",
        ),
        (
            "server/controllers/ProductsController.go",
            "// @RequestRoute(\"/products\")\n// @Public\n// @NeedsAll\n// @yon::Needs(CounterService)\ntype ProductsController struct {\n\tcounter *CounterService\n}\n\n// @GetRoute\nfunc (controller *ProductsController) GetProducts() (any, error) {\n\treturn controller.checkedProducts(controller.counter.Value())\n}\n\nfunc (controller *ProductsController) checkedProducts(value int) (any, error) {\n\tif value != controller.expectedCount() {\n\t\treturn map[string]any{\"error\": \"injection\"}, nil\n\t}\n\treturn CatalogChecksHelper{}.Exercise()\n}\n\nfunc (ProductsController) expectedCount() int { return 42 }\n",
        ),
    ];
}

async fn invoke(root: &std::path::Path, relative: &str) -> serde_json::Value {
    let source = HandlerSource::discover(root, relative).expect("annotated discovery");
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let response = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("go_behaviors", "/products", HttpMethod::Get),
            &HandlerCancellation::default(),
        )
        .await
        .expect("invocation");
    assert!(response.error.is_none(), "{:?}", response.error);
    serde_json::from_str(&response.body.expect("body").data).expect("JSON")
}
