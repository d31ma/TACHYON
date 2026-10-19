#![allow(clippy::expect_used)]

use std::fs;
use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

use crate::support::behavior_layout;

use crate::support::authored_behaviors;

async fn assert_behaviors(extension: &str) {
    if !compiler_available(extension) {
        return;
    }
    let root = tempfile::tempdir().expect("project directory");
    let relative = format!("server/controllers/ProductsController.{extension}");
    authored_behaviors::write(root.path(), extension, "server", extension);
    let source = HandlerSource::discover(root.path(), &relative).expect("behavior discovery");
    let supervisor = HandlerSupervisor::new(HandlerSupervisorOptions {
        default_timeout: Duration::from_mins(2),
        ..HandlerSupervisorOptions::default()
    })
    .expect("supervisor");
    let response = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("behavior-jvm", "/products", HttpMethod::Get),
            &HandlerCancellation::default(),
        )
        .await
        .expect("supervised behavior invocation");
    assert!(
        response.error.is_none(),
        "{extension}: {:?}",
        response.error
    );
    assert_eq!(response.status, 200);
    let value: serde_json::Value =
        serde_json::from_str(&response.body.expect("body").data).expect("JSON body");
    assert_behavior_results(&value);
}

fn compiler_available(extension: &str) -> bool {
    let compiler = match extension {
        "java" => "javac",
        "groovy" => "groovyc",
        _ => "kotlinc",
    };
    let available = Command::new(compiler)
        .arg(if compiler == "groovyc" {
            "--version"
        } else {
            "-version"
        })
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if !available {
        eprintln!(
            "Skipping {extension} behavior runtime: {compiler} unavailable; dedicated JVM gate requires both compilers"
        );
    }
    crate::support::toolchain::Toolchain::checked(compiler, available)
}

fn assert_behavior_results(value: &serde_json::Value) {
    assert_eq!(value["first"], value["cached"]);
    assert_eq!(value["fresh"]["calls"], 2);
    assert_eq!(value["first"]["page"]["offset"], 10);
    assert_eq!(value["remote"]["path"], "/products");
    assert_eq!(value["saved"]["operation"], "save");
    assert_eq!(value["found"]["table"], "products");
    assert_eq!(value["deleted"]["operation"], "delete");
}

#[test]
fn jvm_behavior_signatures_fail_before_execution() {
    for (extension, entry, source) in [
        (
            "java",
            "server/workers/probe.java",
            "@CacheSettings({\"name\":\"catalog\",\"ttlMs\":1000,\"maxBytes\":1000,\"maxEntries\":10})\nclass Handler {\n Behaviors.Context behaviors;\n @Cached\n String load(String id) { return id; }\n}",
        ),
        (
            "kt",
            "server/workers/probe.kt",
            "@CacheSettings({\"name\":\"catalog\",\"ttlMs\":1000,\"maxBytes\":1000,\"maxEntries\":10})\nclass Handler {\n lateinit var behaviors: Behaviors.Context\n @Cached\n suspend fun load(id: String): Any? { return id }\n}",
        ),
        (
            "java",
            "server/controllers/ProbeController.java",
            "@Setup()\nclass Config {\n // static Behaviors.Configuration configure(Behaviors.Execution execution)\n}\n@RequestRoute(\"/probe\")\n@Public\nclass ProbeController {\n @GetRoute\n Object getProbe() { return null; }\n}",
        ),
        (
            "java",
            "server/workers/probe.java",
            "class Handler {\n Object list(@PageDefaults({\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"]}) Object __ty_page) { return __ty_page; }\n}",
        ),
        (
            "kt",
            "server/workers/probe.kt",
            "class Handler {\n fun list(@PageDefaults({\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"]}) __ty_page: Any?): Any? { return __ty_page }\n}",
        ),
    ] {
        let root = tempfile::tempdir().expect("project");
        behavior_layout::write(root.path(), entry, source);
        let failure =
            HandlerSource::discover(root.path(), entry).expect_err("unsupported JVM signature");
        assert!(
            failure
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code.to_string() == "TY1308"),
            "{extension}: {failure:?}"
        );
    }
}

#[test]
fn jvm_reserved_names_in_literals_and_comments_are_inert() {
    for (extension, source) in [
        (
            "java",
            "class Handler {\n String label = \"__ty_page\"; // __ty_behavior_list\n Object list(@PageDefaults({\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"]}) Object input) { return input; }\n}",
        ),
        (
            "kt",
            "object Handler {\n val label: String = \"__ty_page\" // __ty_behavior_list\n fun list(@PageDefaults({\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"]}) input: Any?): Any? { return input }\n}",
        ),
    ] {
        let root = tempfile::tempdir().expect("project");
        let relative = format!("server/workers/probe.{extension}");
        let helper = root
            .path()
            .join(format!("server/helpers/CatalogHelper.{extension}"));
        fs::create_dir_all(helper.parent().expect("parent")).expect("directory");
        let helper_source = source
            .replace("class Handler", "@Helper\nclass CatalogHelper")
            .replace("object Handler", "@Helper\nclass CatalogHelper");
        fs::write(helper, helper_source).expect("behavior helper");
        let path = root.path().join(&relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        let handler = if extension == "kt" {
            "object Handler {}"
        } else {
            "class Handler {}"
        };
        fs::write(path, handler).expect("source");
        HandlerSource::discover(root.path(), &relative).expect("inert reserved-name text");
    }
}

#[tokio::test]
async fn java_shared_annotations_execute_through_supervisor() {
    assert_behaviors("java").await;
}

#[tokio::test]
async fn groovy_shared_annotations_execute_through_supervisor() {
    assert_behaviors("groovy").await;
}

#[tokio::test]
async fn kotlin_shared_annotations_execute_through_supervisor() {
    assert_behaviors("kt").await;
}
