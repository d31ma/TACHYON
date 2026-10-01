#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    DevServer, DevServerOptions, HandlerCancellation, HandlerSource, HandlerSupervisor,
    HandlerSupervisorOptions, ProjectDiscovery,
};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

struct Exchange;

impl Exchange {
    async fn send(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        stream.write_all(request.as_bytes()).await.expect("request");
        let mut bytes = Vec::new();
        tokio::time::timeout(Duration::from_secs(30), stream.read_to_end(&mut bytes))
            .await
            .expect("bounded response")
            .expect("response");
        String::from_utf8(bytes).expect("UTF-8 response")
    }

    async fn without_body(port: u16, method: &str, path: &str) -> String {
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nCookie: theme=dark; size=large\r\nConnection: close\r\n\r\n"
        );
        Self::send(port, &request).await
    }

    async fn with_body(port: u16, method: &str, path: &str, body: &str) -> String {
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nX-Tenant: north\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        Self::send(port, &request).await
    }
}

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, source).expect("source");
}

fn program_available(program: &str) -> bool {
    std::process::Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn node_available() -> bool {
    program_available("node")
}

struct ControllerSource;

struct NestedYonRoleCheck;

impl NestedYonRoleCheck {
    const CONTROLLER_PATH: &'static str = "server/controllers/api/OrdersController.js";
    const TOTAL_PATH: &'static str = "server/services/state/numbers/TotalService.js";
    const CONTROLLER: &'static str = r#"@RequestRoute("/nested/orders")
@Public
@Needs(TotalService)
class OrdersController {
    constructor(total) { this.total = total; }

    @GetRoute
    getOrders() { return this.total.calculate(6); }
}
"#;
    const TOTAL: &'static str = r"import { InvoiceLabel } from '../../../shared/text/InvoiceLabel.js';
@Service
@Needs(RateService)
class TotalService {
    constructor(rate) { this.rate = rate; }
    calculate(value) { return { total: value * this.rate.value(), label: InvoiceLabel.text() }; }
}
";
    const RATE: &'static str = "@Service\nclass RateService { value() { return 7; } }\n";
    const LABEL: &'static str =
        "export class InvoiceLabel { static text() { return 'shared-rebased'; } }\n";

    fn write(root: &Path) {
        write(root, Self::CONTROLLER_PATH, Self::CONTROLLER);
        write(root, Self::TOTAL_PATH, Self::TOTAL);
        write(root, "server/services/tax/RateService.js", Self::RATE);
        write(root, "server/shared/text/InvoiceLabel.js", Self::LABEL);
    }

    fn check_paths(root: &Path) {
        let captured = HandlerSource::discover(root, Self::CONTROLLER_PATH)
            .expect("capture nested controller and realm providers");
        assert_eq!(captured.relative_path(), Self::CONTROLLER_PATH);
        let project = ProjectDiscovery::discover(root).expect("nested route discovery");
        let manifest = project.route_graph().manifest();
        assert_eq!(manifest.routes.len(), 1);
        assert_eq!(manifest.routes[0].route, "/nested/orders");
        assert_eq!(manifest.routes[0].handlers[0].source, Self::CONTROLLER_PATH);
    }

    fn check_missing_marker(root: &Path) {
        write(
            root,
            Self::TOTAL_PATH,
            &Self::TOTAL.replace("@Service\n", ""),
        );
        let failure = HandlerSource::discover(root, Self::CONTROLLER_PATH)
            .expect_err("a real nested provider must keep its formal role");
        assert!(
            failure
                .diagnostics()
                .iter()
                .any(|found| found.code.number() == 1363)
        );
        assert!(failure.to_string().contains(Self::TOTAL_PATH), "{failure}");
    }
}

#[tokio::test]
async fn a_nested_yon_controller_executes_realm_providers_and_rebased_imports() {
    let root = tempfile::tempdir().expect("project");
    NestedYonRoleCheck::write(root.path());
    NestedYonRoleCheck::check_paths(root.path());
    let (port, stop, running) = serve_orders(root.path()).await;
    let response = Exchange::without_body(port, "GET", "/nested/orders").await;
    stop.send(()).expect("stop owned server");
    tokio::time::timeout(Duration::from_secs(20), running)
        .await
        .expect("bounded shutdown")
        .expect("server task")
        .expect("server shutdown");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let (_, body) = response.split_once("\r\n\r\n").expect("HTTP body");
    let body: serde_json::Value = serde_json::from_str(body).expect("actual JSON response");
    assert_eq!(
        body,
        serde_json::json!({"total":42,"label":"shared-rebased"})
    );
    NestedYonRoleCheck::check_missing_marker(root.path());
}

impl ControllerSource {
    const JAVASCRIPT_ORDER: &str = r"/**
 * @typedef {Object} Order
 * @property {string} name
 * @property {number} [count]
 * @property {string} [tenant]
 */
export {}
";
    const JAVASCRIPT: &str = r#"@RequestRoute("/orders")
@Public
class OrdersController {
  static _SECRET_ORDER = 4

  @GetRoute
  getOrders() {
    return [{ id: "1" }]
  }

  /**
   * @param {number} id
   * @param {string} [view]
   * @param {string} theme
   */
  @GetRoute("/{id}")
  getOrderById(@FromPath id, @FromQuery("view") view, @FromCookie("theme") theme) {
    return { id, view, theme }
  }

  /**
   * @param {Order} order
   * @param {string} tenant
   * @returns {Order}
   */
  @PostRoute
  @ResponseStatus(201)
  postOrder(@FromBody order, @FromHeader("x-tenant") tenant) {
    return { name: order.name, tenant }
  }

  /**
   * @param {number} id
   * @returns {Order}
   */
  @PatchRoute("/{id}")
  patchOrder(@FromPath id) {
    return this.#orderFor(id)
  }

  #orderFor(id) {
    if (id === OrdersController._SECRET_ORDER) return { name: 'lamp', PRIVATE_SECRET: 'not exposed' }
    return { count: id }
  }

  @DeleteRoute("/{id}")
  deleteOrder(request) {
    return { route: new URL(request.url).pathname, method: request.method }
  }
}
"#;
}

async fn serve_orders(
    root: &Path,
) -> (
    u16,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<Result<(), tachyon_core::Failure>>,
) {
    let project = ProjectDiscovery::discover(root).expect("discovery");
    let server = DevServer::bind_project(
        &project,
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(server.run_until(async {
        let _ = stopped.await;
    }));
    (port, stop, running)
}

#[tokio::test]
async fn a_javascript_controller_answers_its_routes() {
    if !node_available() {
        eprintln!("skipping: node is not installed");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/controllers/OrdersController.js",
        ControllerSource::JAVASCRIPT,
    );
    write(
        root.path(),
        "server/models/Order.js",
        ControllerSource::JAVASCRIPT_ORDER,
    );
    let (port, stop, running) = serve_orders(root.path()).await;

    let list = Exchange::without_body(port, "GET", "/orders").await;
    assert!(list.starts_with("HTTP/1.1 200"), "{list}");
    assert!(list.contains(r#"[{"id":"1"}]"#), "{list}");

    let one = Exchange::without_body(port, "GET", "/orders/42?view=full").await;
    assert!(one.starts_with("HTTP/1.1 200"), "{one}");
    assert!(
        one.contains(r#"{"id":42,"view":"full","theme":"dark"}"#),
        "{one}"
    );

    let not_a_number = Exchange::without_body(port, "GET", "/orders/abc").await;
    assert!(not_a_number.starts_with("HTTP/1.1 400"), "{not_a_number}");
    assert!(
        not_a_number.contains("The path value 'id' must be a number."),
        "{not_a_number}"
    );

    let wrong_body = Exchange::with_body(port, "POST", "/orders", r#"{"count":1}"#).await;
    assert!(wrong_body.starts_with("HTTP/1.1 400"), "{wrong_body}");
    assert!(
        wrong_body.contains("'body.name' is required"),
        "{wrong_body}"
    );

    let wrong_answer = Exchange::with_body(port, "PATCH", "/orders/3", "{}").await;
    assert!(wrong_answer.starts_with("HTTP/1.1 500"), "{wrong_answer}");
    assert!(
        wrong_answer.contains("The answer does not match the declared return type."),
        "{wrong_answer}"
    );

    let extra_answer = Exchange::with_body(port, "PATCH", "/orders/4", "{}").await;
    assert!(extra_answer.starts_with("HTTP/1.1 500"), "{extra_answer}");
    assert!(!extra_answer.contains("PRIVATE_SECRET"), "{extra_answer}");
    assert!(!extra_answer.contains("not exposed"), "{extra_answer}");

    let created = Exchange::with_body(port, "POST", "/orders", r#"{"name":"lamp"}"#).await;
    assert!(created.starts_with("HTTP/1.1 201"), "{created}");
    assert!(
        created.contains(r#"{"name":"lamp","tenant":"north"}"#),
        "{created}"
    );

    let removed = Exchange::without_body(port, "DELETE", "/orders/7").await;
    assert!(
        removed.contains(r#"{"route":"/orders/7","method":"DELETE"}"#),
        "{removed}"
    );

    let head = Exchange::without_body(port, "HEAD", "/orders").await;
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");

    let refused = Exchange::with_body(port, "PUT", "/orders", "{}").await;
    assert!(refused.starts_with("HTTP/1.1 405"), "{refused}");
    assert!(
        refused
            .to_ascii_lowercase()
            .contains("allow: get, head, options, post"),
        "{refused}"
    );

    let options = Exchange::without_body(port, "OPTIONS", "/orders/42").await;
    assert!(options.starts_with("HTTP/1.1 200"), "{options}");
    assert!(options.contains(r#""path":"/orders/{id}""#), "{options}");
    assert!(
        options.contains(r#""parameters":{"id":{"type":"number"}}"#),
        "{options}"
    );
    assert!(
        options
            .to_ascii_lowercase()
            .contains("allow: delete, get, head, options, patch"),
        "{options}"
    );
    let order_options = Exchange::without_body(port, "OPTIONS", "/orders").await;
    assert!(order_options.contains(r#""Order":{"#), "{order_options}");
    assert!(
        order_options.contains(r#""additionalProperties":false"#),
        "{order_options}"
    );

    let broken = Exchange::with_body(port, "POST", "/orders", "not json").await;
    assert!(broken.starts_with("HTTP/1.1 400"), "{broken}");

    let missing = Exchange::without_body(port, "GET", "/customers").await;
    assert!(missing.starts_with("HTTP/1.1 404"), "{missing}");

    stop.send(()).expect("stop");
    tokio::time::timeout(Duration::from_secs(20), running)
        .await
        .expect("bounded shutdown")
        .expect("server task")
        .expect("server shutdown");
}

#[test]
fn the_route_manifest_lists_controller_paths_and_methods() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/controllers/OrdersController.js",
        ControllerSource::JAVASCRIPT,
    );
    write(
        root.path(),
        "server/models/Order.js",
        ControllerSource::JAVASCRIPT_ORDER,
    );
    let project = ProjectDiscovery::discover(root.path()).expect("discovery");
    let manifest = project.route_graph().manifest();
    let routes: Vec<&str> = manifest
        .routes
        .iter()
        .map(|entry| entry.route.as_str())
        .collect();
    assert_eq!(routes, ["/orders", "/orders/_id"]);
    assert_eq!(manifest.routes[0].methods.len(), 4);
    assert_eq!(manifest.routes[1].methods.len(), 5);
}

#[test]
fn two_controllers_cannot_answer_the_same_route() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/controllers/OrdersController.js",
        ControllerSource::JAVASCRIPT,
    );
    write(
        root.path(),
        "server/models/Order.js",
        ControllerSource::JAVASCRIPT_ORDER,
    );
    write(
        root.path(),
        "server/controllers/ShopController.js",
        "@RequestRoute(\"/orders\")\n@Public\nclass ShopController {\n  @GetRoute\n  getAll() {\n    return []\n  }\n}\n",
    );
    let failure = ProjectDiscovery::discover(root.path()).expect_err("duplicate route");
    assert!(
        failure
            .to_string()
            .contains("Two controllers answer GET /orders"),
        "{failure}"
    );
}

#[test]
fn a_route_method_name_starts_with_its_verb() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/controllers/OrdersController.js",
        "@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n  @PostRoute\n  getOrders() {\n    return []\n  }\n}\n",
    );
    let failure = ProjectDiscovery::discover(root.path()).expect_err("verb prefix");
    let message = failure.to_string();
    assert!(
        message.contains(
            "@PostRoute means this method answers POST requests, so its name must start with 'post'"
        ),
        "{message}"
    );
}

#[tokio::test]
async fn a_typescript_controller_uses_a_shared_service() {
    if !program_available("bun") {
        eprintln!("skipping: bun is not installed");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/services/OrdersService.ts",
        "@Service\nexport default class OrdersService {\n  find(id: string): { id: string, name: string } {\n    return { id, name: 'lamp' }\n  }\n}\n",
    );
    write(
        root.path(),
        "server/controllers/OrdersController.ts",
        "@RequestRoute(\"/orders\")\n@Public\n@NeedsAll\nexport default class OrdersController {\n  readonly ordersService!: OrdersService\n\n  @GetRoute(\"/{id}\")\n  getOrder(@FromPath id: string) {\n    return this.ordersService.find(id)\n  }\n}\n",
    );
    let source = HandlerSource::discover(root.path(), "server/controllers/OrdersController.ts")
        .expect("controller");
    let mut options = HandlerSupervisorOptions::default();
    options.runtimes.javascript = "bun".into();
    let supervisor = HandlerSupervisor::new(options).expect("supervisor");
    let mut request = HandlerRequest::route("typescript", "/orders/_id", HttpMethod::Get);
    request.arguments = Some(vec![serde_json::json!("9")]);
    let response = supervisor
        .invoke(&source, &request, &HandlerCancellation::default())
        .await
        .expect("invocation");
    assert!(response.error.is_none(), "{:?}", response.error);
    let body = response.body.expect("body").data;
    assert_eq!(body, r#"{"id":"9","name":"lamp"}"#);
}
