#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    BuildOptions, HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
    ProjectDiscovery, WebCompiler,
};

use crate::support::behavior_layout;

use crate::support::authored_behaviors;

fn write(root: &Path, relative: &str, source: &str) {
    behavior_layout::write(root, relative, source);
}

#[tokio::test]
async fn shared_behaviors_execute_in_direct_and_injected_yon_js_and_ts() {
    for extension in ["js", "ts"] {
        for annotated in [false, true] {
            assert_yon_behaviors(extension, annotated).await;
        }
    }
}

#[tokio::test]
async fn plain_controllers_run_in_yon_js_and_ts() {
    for (extension, entry) in [
        (
            "js",
            "@RequestRoute(\"/products\")\n@Public\nexport class ProductsController {\n@GetRoute\nasync getProducts(request) { return { plain: true, method: request.method }; } }",
        ),
        (
            "ts",
            "@RequestRoute(\"/products\")\n@Public\nexport class ProductsController {\n@GetRoute\nasync getProducts(request: { method: string }) { return { plain: true, method: request.method }; } }",
        ),
    ] {
        let root = tempfile::tempdir().expect("project");
        let relative = format!("server/controllers/ProductsController.{extension}");
        write(root.path(), &relative, entry);
        let body = invoke_products(root.path(), &relative, extension).await;
        assert_eq!(
            body,
            serde_json::json!({"plain": true, "method": "GET"}),
            "{extension}"
        );
    }
}

async fn assert_yon_behaviors(extension: &str, annotated: bool) {
    let root = tempfile::tempdir().expect("project");
    let relative = format!("server/controllers/ProductsController.{extension}");
    authored_behaviors::write(root.path(), "js", "server", extension);
    let entry = if annotated {
        "\n@Service()\nclass Counter { value = 42; }\n@Service()\n@NeedsAll\n@Needs(Counter)\nclass ServiceWithGeneratedConstructor {\n counter;\n}\n@RequestRoute(\"/products\")\n@Public\n@Needs(ServiceWithGeneratedConstructor)\nclass ProductsController {\n constructor(service) { this.service = service; }\n @GetRoute\n async getProducts() { this.#requireInjected(); return BehaviorChecksDelegate.exerciseBehaviors(new AbortController().signal); }\n #requireInjected() { if (this.service.counter.value !== 42) throw new Error('constructor injection failed'); }\n }"
    } else {
        "\n@RequestRoute(\"/products\")\n@Public\nclass ProductsController {\n@GetRoute\nasync getProducts() { return BehaviorChecksDelegate.exerciseBehaviors(new AbortController().signal); } }"
    };
    write(root.path(), &relative, entry);
    let body = invoke_products(root.path(), &relative, extension).await;
    assert_behavior_results(&body);
}

async fn invoke_products(root: &Path, relative: &str, extension: &str) -> serde_json::Value {
    let source = HandlerSource::discover(root, relative).expect("discovery");
    let mut options = HandlerSupervisorOptions::default();
    if extension == "ts" {
        options.runtimes.javascript = "bun".into();
    }
    let supervisor = HandlerSupervisor::new(options).expect("supervisor");
    let response = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("behavior_1", "/products", HttpMethod::Get),
            &HandlerCancellation::default(),
        )
        .await
        .expect("invocation");
    assert!(
        response.error.is_none(),
        "{extension}: {:?}",
        response.error
    );
    serde_json::from_str(&response.body.expect("body").data).expect("JSON")
}

fn assert_behavior_results(body: &serde_json::Value) {
    assert_eq!(body["first"], body["cached"]);
    assert_eq!(body["fresh"]["calls"], 2);
    assert_eq!(body["page"]["offset"], 10);
    assert_eq!(body["remote"]["path"], "/products");
    assert_eq!(body["declared"]["query"]["page"], 2);
    assert_eq!(body["declared"]["headers"]["X-Tenant"], "acme");
    assert_eq!(body["saved"]["operation"], "save");
    assert_eq!(body["found"]["table"], "products");
    assert_eq!(body["deleted"]["operation"], "delete");
    assert_configuration_results(body);
    assert_validation_results(body);
}

fn assert_configuration_results(body: &serde_json::Value) {
    assert_eq!(
        body["configuration"]["settings"],
        serde_json::json!({"label":"Products","enabled":true,"limit":12,"ratio":1.5})
    );
    assert_eq!(body["configuration"]["inactive"], "inactive-configuration");
    assert_eq!(
        body["configuration"]["selectionErrors"],
        serde_json::json!(["inactive-configuration", "inactive-configuration"])
    );
    assert_eq!(
        body["configuration"]["propertyError"],
        "configuration-properties"
    );
    assert_eq!(body["configuration"]["calls"], 1);
}

fn assert_validation_results(body: &serde_json::Value) {
    assert_eq!(
        body["validation"]["input"],
        serde_json::json!({"id":7,"name":"é","price":5,"extra":true,"validated":true})
    );
    assert_eq!(
        body["validation"]["constraintPaths"],
        serde_json::json!(["id.notNull", "name.size", "price.min", "price.max"])
    );
    assert_eq!(body["validation"]["path"], "name.notBlank");
    assert_eq!(body["validation"]["calls"], 1);
    assert_eq!(body["validation"]["copyPreserved"], true);
}

#[test]
fn tac_annotations_compile_without_executing_configuration_or_rendering_views() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/pages/tac.html",
        "<page-view hydrate=\"load\" />",
    );
    write(
        root.path(),
        "client/components/page/view/tac.html",
        "<main>{pageView.value}</main>",
    );
    authored_behaviors::write(root.path(), "js", "client", "js");
    write(
        root.path(),
        "client/manifest.json",
        r#"{"tac":{"visibility":{"check":false}}}"#,
    );
    let source = "@Service()\nexport default class PageViewService {\n value = 'client';\n @OnStart()\n initialize() { throw new Error('build executed client'); }\n}";
    write(root.path(), "client/services/PageViewService.js", source);
    let project = ProjectDiscovery::discover(root.path()).expect("discovery");
    let build = WebCompiler::build_project(&project, &BuildOptions::default()).expect("build");
    let _ = build;
    let html = fs::read_to_string(root.path().join("dist/index.html")).expect("bootstrap");
    assert!(!html.contains("<main>client</main>"));
    let script = fs::read_to_string(
        root.path()
            .join("dist/.tachyon/services/PageViewService.js"),
    )
    .expect("client module");
    assert!(!script.contains("@CacheSettings("));
    assert!(script.contains("__tachyon_create_behaviors__"));
}

#[test]
fn an_import_from_tachyon_behaviors_is_refused_in_tac_and_yon() {
    let import = "import { Service } from 'tachyon:behaviors';\n";
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/pages/tac.html",
        "<page-view hydrate=\"load\" />",
    );
    write(
        root.path(),
        "client/components/page/view/tac.html",
        "<main>{pageView.value}</main>",
    );
    let service =
        format!("{import}@Service()\nexport default class PageViewService {{ value = 'client'; }}");
    write(root.path(), "client/services/PageViewService.js", &service);
    let failure = ProjectDiscovery::discover(root.path())
        .and_then(|project| WebCompiler::build_project(&project, &BuildOptions::default()))
        .expect_err("the Tac import is refused")
        .to_string();
    assert!(failure.contains("TY1308"), "{failure}");
    assert!(
        failure.contains("client/services/PageViewService.js:1: Tachyon annotations and types are global; delete this import"),
        "{failure}"
    );
    let root = tempfile::tempdir().expect("project");
    let controller = format!(
        "{import}@RequestRoute(\"/products\")\nexport class ProductsController {{\n@GetRoute\ngetProducts() {{ return {{}}; }}\n}}"
    );
    let relative = "server/controllers/ProductsController.js";
    write(root.path(), relative, &controller);
    let failure = ProjectDiscovery::discover(root.path())
        .map(|_| ())
        .and_then(|()| HandlerSource::discover(root.path(), relative).map(|_| ()))
        .expect_err("the Yon import is refused")
        .to_string();
    assert!(failure.contains("TY1308"), "{failure}");
    assert!(
        failure.contains("server/controllers/ProductsController.js:1: Tachyon annotations"),
        "{failure}"
    );
}

#[test]
fn invalid_native_tac_behavior_policies_fail_before_native_toolchain_execution() {
    for (extension, registration, marker) in [
        ("swift", "@Service", "@CacheSettings()"),
        ("kt", "@Service", "@CacheSettings()"),
        ("cs", "[Service]", "[Cache]"),
        ("rs", "#[Service]", "#[tac::cache]"),
    ] {
        let root = tempfile::tempdir().expect("project");
        write(root.path(), "client/pages/tac.html", "<page-view />");
        write(
            root.path(),
            "client/components/page/view/tac.html",
            "<main>Client</main>",
        );
        write(
            root.path(),
            "client/services/PageViewService.js",
            "@Service()\nexport default class PageViewService {}",
        );
        let declaration = match extension {
            "rs" => "struct PageViewService {}",
            _ => "class PageViewService {}",
        };
        write(
            root.path(),
            &format!("client/services/PageViewService.{extension}"),
            &format!("{registration}\n{marker}\n{declaration}"),
        );
        let failure = WebCompiler::build(root.path(), &BuildOptions::default())
            .expect_err("invalid native behavior policy");
        assert!(
            failure
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code.to_string() == "TY1308"),
            "{extension}: {failure:?}"
        );
        assert!(!root.path().join("dist").exists());
    }
}
