#![allow(clippy::expect_used)]

use std::{fs, path::Path};
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{BuildOptions, HandlerSource, ProjectDiscovery, WebCompiler};
use tachyon_core::{HandlerCancellation, HandlerSupervisor, HandlerSupervisorOptions};

fn write_shared_roles(root: &Path) {
    write(
        root,
        "server/services/OrdersService.js",
        "@Service\nclass OrdersService { total() { return 7; } }",
    );
    write(
        root,
        "server/helpers/AuditHelper.js",
        "@Helper\nclass AuditHelper { stamp(value) { return value + 1; } }",
    );
    for (route, controller) in [
        ("orders", "OrdersController"),
        ("products", "ProductsController"),
    ] {
        write(
            root,
            &format!("server/controllers/{controller}.js"),
            &format!(
                "@RequestRoute(\"/{route}\")\n@Public\n@NeedsAll()\nclass {controller} {{\n ordersService;\n auditHelper;\n @GetRoute\n get() {{ return {{total:this.auditHelper.stamp(this.ordersService.total())}}; }}\n}}"
            ),
        );
    }
}

fn write_basket_roles(root: &Path, constructor: &str, fields: &str) {
    write(
        root,
        "client/pages/tac.html",
        "<basket-view hydrate=\"load\" />",
    );
    write(
        root,
        "client/components/basket/view/tac.html",
        "<main>{basket.total}</main>",
    );
    write(
        root,
        "client/services/BasketService.js",
        &format!(
            "@Service()\n{constructor}\nexport default class BasketService {{\n{fields}\ntotal = 0;\n}}"
        ),
    );
    write(
        root,
        "client/helpers/BasketItemsHelper.js",
        "@Helper()\nclass BasketItemsHelper {\n_items = [];\n}",
    );
    write(
        root,
        "client/helpers/PricingHelper.js",
        "@Helper()\nclass PricingHelper {\n_price(items) { return items.length; }\n}",
    );
    write(
        root,
        "client/clients/ProductsClient.js",
        "@HTTPClient({\"adapter\":\"api\"})\nclass ProductsClient {\n@GetRequest(\"/api/products\")\nasync all() {}\n}",
    );
}

fn write_page_view_roles(root: &Path) {
    write(
        root,
        "client/pages/tac.html",
        "<page-view hydrate=\"load\" />",
    );
    write(
        root,
        "client/components/page/view/tac.html",
        "<main>{pageView.value}</main>",
    );
    write(
        root,
        "client/services/PageViewService.js",
        "@Service()\nexport default class PageViewService { value = 'client'; }",
    );
    write(
        root,
        "client/entities/MoneyEntity.js",
        "@Entity({\"adapter\":\"money\"})\n@Table(\"money\")\nclass MoneyEntity {\n  @Id\n  id;\n}",
    );
}

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, source).expect("source");
}

#[test]
fn inline_roles_fail_before_runtime_selection() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/workers/money.js",
        "@RequestRoute(\"/money\")\n@Public\nclass MoneyController {\n  @GetRoute\n  getMoney() { return 42; }\n}",
    );
    let failure = HandlerSource::discover(root.path(), "server/workers/money.js")
        .expect_err("role files are mandatory");
    assert!(failure.to_string().contains("MoneyController"), "{failure}");
}

#[test]
fn orphan_module_manifests_are_rejected_during_project_discovery() {
    let root = tempfile::tempdir().expect("project");
    write(root.path(), "client/pages/tac.html", "<app-view />");
    write(
        root.path(),
        "client/components/app/view/tac.html",
        "<main>Public</main>",
    );
    write(root.path(), "client/shared/old/tac.modules.json", "{}");
    let failure = ProjectDiscovery::discover(root.path()).expect_err("orphan manifest");
    assert!(
        failure.to_string().contains("no longer supported"),
        "{failure}"
    );
    assert!(
        failure
            .to_string()
            .contains("client/shared/old/tac.modules.json"),
        "{failure}"
    );
}

#[tokio::test]
async fn discovered_roles_preserve_handler_routing_and_captured_execution() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/services/MoneyService.js",
        "@Service\nclass MoneyService { amount() { return 42; } }",
    );
    write(
        root.path(),
        "server/controllers/MoneyController.js",
        "@RequestRoute(\"/money\")\n@Public\n@NeedsAll()\nclass MoneyController {\n static otherService;\n _offset = 0;\n moneyService;\n @GetRoute\n getMoney() { return {amount:this.moneyService.amount() + this._offset}; }\n }",
    );
    let project = ProjectDiscovery::discover(root.path()).expect("discovered roles");
    assert_eq!(project.route_graph().routes().len(), 1);
    assert_eq!(project.route_graph().routes()[0].route(), "/money");
    let source = HandlerSource::discover(root.path(), "server/controllers/MoneyController.js")
        .expect("capture handler");
    write(
        root.path(),
        "server/services/MoneyService.js",
        "throw new Error('reopened module');",
    );
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let response = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("module_layout_1", "/money", HttpMethod::Get),
            &HandlerCancellation::default(),
        )
        .await
        .expect("captured invocation");
    assert!(response.error.is_none(), "{:?}", response.error);
    let body: serde_json::Value =
        serde_json::from_str(&response.body.expect("body").data).expect("JSON");
    assert_eq!(body, serde_json::json!({"amount":42}));
}

fn throw_when_loaded(source: &str, class_opening: &str) -> String {
    let throwing_opening = format!(
        "{class_opening}\n static loaded = (() => {{ throw new Error('discovery executed {class_opening}'); }})();"
    );
    assert!(source.contains(class_opening), "{source}");
    source.replace(class_opening, &throwing_opening)
}

#[test]
fn required_args_dependencies_are_validated_before_application_execution() {
    for (service, controller, expected) in [
        (
            "@Service\nclass MoneyService {}",
            "@RequestRoute(\"/money\")\n@Public\n@NeedsAll()\nclass MoneyController {\n missingService;\n @GetRoute\n getMoney() { return 42; }\n}",
            "needs field 'missingService'",
        ),
        (
            "@Service\n@NeedsAll()\nclass MoneyService {\n moneyService;\n}",
            "@RequestRoute(\"/money\")\n@Public\n@NeedsAll()\nclass MoneyController {\n moneyService;\n @GetRoute\n getMoney() { return 42; }\n}",
            "cycle",
        ),
        (
            "@Service\nclass MoneyService {}",
            "@RequestRoute(\"/money\")\n@Public\n@NeedsAll()\nclass MoneyController {\n moneyService;\n constructor() {}\n @GetRoute\n getMoney() { return 42; }\n}",
            "builds this class for you",
        ),
        (
            "@Service\n@Lifetime('request')\nclass MoneyService {}",
            "@RequestRoute(\"/money\")\n@Public\n@Lifetime('singleton')\n@NeedsAll()\nclass MoneyController {\n moneyService;\n @GetRoute\n getMoney() { return 42; }\n}",
            "captures request-scoped",
        ),
    ] {
        let root = tempfile::tempdir().expect("project");
        write(
            root.path(),
            "server/services/MoneyService.js",
            &throw_when_loaded(service, "class MoneyService {"),
        );
        write(
            root.path(),
            "server/controllers/MoneyController.js",
            &throw_when_loaded(controller, "class MoneyController {"),
        );
        let failure = HandlerSource::discover(root.path(), "server/controllers/MoneyController.js")
            .expect_err("invalid inferred dependency graph must fail discovery");
        assert!(failure.to_string().contains(expected), "{failure}");
        assert!(
            !failure.to_string().contains("discovery executed"),
            "{failure}"
        );
        assert!(!root.path().join("dist").exists());
    }
}

#[test]
fn a_role_beside_a_controller_names_its_shared_destination() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/controllers/services/OrdersService.js",
        "@Service\nclass OrdersService { total() { return 1; } }",
    );
    let failure = ProjectDiscovery::discover(root.path()).expect_err("route-side role");
    let rendered = failure.to_string();
    assert!(rendered.contains("TY1310"), "{rendered}");
    assert!(
        rendered
            .contains("@Service type 'OrdersService' sits outside its realm's shared role folder"),
        "{rendered}"
    );
    assert!(
        rendered.contains("server/services/OrdersService.js"),
        "{rendered}"
    );
}

#[tokio::test]
async fn shared_yon_roles_compose_into_every_controller() {
    let root = tempfile::tempdir().expect("project");
    write_shared_roles(root.path());
    let project = ProjectDiscovery::discover(root.path()).expect("shared roles");
    assert_eq!(project.route_graph().routes().len(), 2);
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    for (index, (route, controller)) in [
        ("/orders", "OrdersController"),
        ("/products", "ProductsController"),
    ]
    .into_iter()
    .enumerate()
    {
        let entry = format!("server/controllers/{controller}.js");
        let source = HandlerSource::discover(root.path(), &entry).expect("capture handler");
        let response = supervisor
            .invoke(
                &source,
                &HandlerRequest::route(format!("shared_roles_{index}"), route, HttpMethod::Get),
                &HandlerCancellation::default(),
            )
            .await
            .expect("captured invocation");
        assert!(response.error.is_none(), "{route}: {:?}", response.error);
        let body: serde_json::Value =
            serde_json::from_str(&response.body.expect("body").data).expect("JSON");
        assert_eq!(body, serde_json::json!({"total":8}), "{route}");
    }
}

#[test]
fn a_tac_service_receives_helpers_and_injected_providers() {
    for (constructor, fields) in [
        (
            "@NeedsAll()",
            "basketItemsHelper;\npricingHelper;\nproductsClient;",
        ),
        (
            "@Needs(BasketItemsHelper, PricingHelper, ProductsClient)",
            "basketItemsHelper;\npricingHelper;\nproductsClient;",
        ),
    ] {
        let root = tempfile::tempdir().expect("project");
        write_basket_roles(root.path(), constructor, fields);
        let project = ProjectDiscovery::discover(root.path()).expect("client roles");
        WebCompiler::build_project(&project, &BuildOptions::default()).expect("compile services");
        let module =
            fs::read_to_string(root.path().join("dist/.tachyon/services/BasketService.js"))
                .expect("service module");
        for role in [
            "class BasketItemsHelper",
            "class PricingHelper",
            "class ProductsClient",
        ] {
            assert!(module.contains(role), "{constructor}: {role}");
        }
        assert!(
            module.contains("constructor(basketItemsHelper, pricingHelper, productsClient)"),
            "{constructor}: {module}"
        );
        assert!(
            module.contains(
                "new BasketService(resolve('BasketItemsHelper'),resolve('PricingHelper'),resolve('ProductsClient'))"
            ),
            "{constructor}: {module}"
        );
        assert!(
            !module.contains("@Helper") && !module.contains("@Needs"),
            "{module}"
        );
    }
}

#[test]
fn tac_module_changes_are_captured_and_invalidate_output() {
    let root = tempfile::tempdir().expect("project");
    write_page_view_roles(root.path());
    let project = ProjectDiscovery::discover(root.path()).expect("capture modules");
    write(
        root.path(),
        "client/entities/MoneyEntity.js",
        "throw new Error('not captured');",
    );
    WebCompiler::build_project(&project, &BuildOptions::default())
        .expect("compile captured module");
    let script = fs::read_to_string(
        root.path()
            .join("dist/.tachyon/services/PageViewService.js"),
    )
    .expect("module output");
    assert!(script.contains("class MoneyEntity"));
    assert!(!script.contains("not captured"));
    assert!(!script.contains("@Entity"));
    write(
        root.path(),
        "client/entities/MoneyEntity.js",
        "@Entity({\"adapter\":\"money\"})\n@Table(\"money_updated\")\nclass MoneyEntity {\n  @Id\n  id;\n}",
    );
    let updated = ProjectDiscovery::discover(root.path()).expect("capture updated module");
    WebCompiler::build_project(&updated, &BuildOptions::default()).expect("compile updated module");
    let changed = fs::read_to_string(
        root.path()
            .join("dist/.tachyon/services/PageViewService.js"),
    )
    .expect("updated output");
    assert_ne!(
        script, changed,
        "module metadata changes must invalidate output"
    );
    assert!(changed.contains("money_updated"));
}

#[test]
fn a_role_annotation_in_client_models_fails_project_discovery() {
    let root = tempfile::tempdir().expect("project");
    write(root.path(), "client/pages/tac.html", "<app-view />");
    write(
        root.path(),
        "client/components/app/view/tac.html",
        "<main>Public</main>",
    );
    write(
        root.path(),
        "client/models/GuideService.ts",
        "@Service\nexport default class GuideService {}\n",
    );
    let failure = ProjectDiscovery::discover(root.path()).expect_err("role in models");
    let rendered = failure.to_string();
    assert!(rendered.contains("TY1310"), "{rendered}");
    assert!(
        rendered.contains("client/services/GuideService.ts"),
        "{rendered}"
    );
    write(
        root.path(),
        "client/models/GuideService.ts",
        "export interface Guide { title: string }\nexport const empty = {}\n",
    );
    let failure = ProjectDiscovery::discover(root.path()).expect_err("value in models");
    assert!(
        failure
            .to_string()
            .contains("A models file declares only interfaces and type aliases"),
        "{failure}"
    );
}

#[test]
fn a_models_file_is_captured_but_never_shipped() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/pages/tac.html",
        "<guide-view hydrate=\"load\" />",
    );
    write(
        root.path(),
        "client/components/guide/view/tac.html",
        "<main>{guideView.title}</main>",
    );
    write(
        root.path(),
        "client/services/GuideViewService.js",
        "@Service()\nexport default class GuideViewService { title = 'Guide'; }",
    );
    write(
        root.path(),
        "client/models/Guide.ts",
        "export interface Guide {\n  title: string\n}\n",
    );
    write(
        root.path(),
        "client/models/Shelf.rs",
        "pub struct Shelf {\n    pub size: i64,\n}\n",
    );
    write(
        root.path(),
        "client/models/Shelf.dart",
        "class Shelf {\n  final int size;\n\n  const Shelf(this.size);\n}\n",
    );
    write(
        root.path(),
        "server/models/Shelf.py",
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Shelf:\n    size: int\n",
    );
    let project = ProjectDiscovery::discover(root.path()).expect("types in models");
    WebCompiler::build_project(&project, &BuildOptions::default()).expect("build");
    assert_models_absent(root.path());
}

fn assert_models_absent(root: &Path) {
    let mut pending = vec![root.join("dist")];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("output").flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            assert!(
                !path.to_string_lossy().contains("models"),
                "{}",
                path.display()
            );
            let bytes = fs::read(&path).expect("output file");
            let text = String::from_utf8_lossy(&bytes);
            assert!(
                !text.contains("interface Guide") && !text.contains("Shelf"),
                "{}",
                path.display()
            );
        }
    }
}
