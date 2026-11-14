use super::{ClientViewRenderer, ComponentRegistry, Scope, ServiceRegistry, TemplateFrontend};
use crate::Failure;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

struct RemoteViews;

impl RemoteViews {
    const PANEL: &'static str = "client/components/orders/panel/tac.html";
    const REMOTE: &'static str = "@HTTPClient\nexport default class OrdersClient {\n  @GetRequest(\"/api/orders\")\n  async list() {}\n}\n";
    const WRAPPER: &'static str = "@Service\nexport default class OrderListService {\n  /** @type {OrdersClient} */\n  @Needs\n  ordersClient\n  orders = []\n  async load() {\n    this.orders = await this.ordersClient.list();\n  }\n}\n";
    const PLAIN: &'static str =
        "@Service\nexport default class ShellService {\n  title = 'Orders'\n}\n";

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, source).expect("source");
    }

    fn project(panel: &str, service: (&str, &str)) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        Self::write(root.path(), "client/remotes/OrdersClient.js", Self::REMOTE);
        Self::write(root.path(), service.0, service.1);
        Self::write(root.path(), Self::PANEL, panel);
        root
    }

    fn render(panel: &str, service: (&str, &str)) -> Result<(), Failure> {
        let root = Self::project(panel, service);
        let components = ComponentRegistry::discover(root.path()).expect("components");
        let services = ServiceRegistry::discover(root.path()).expect("services");
        let program = TemplateFrontend::compile(
            "<orders-panel />",
            "client/pages/tac.html",
            &components.names(),
        )
        .expect("page");
        ClientViewRenderer::new(&components, &services, &BTreeSet::new())
            .render(&program, "index.html", &Scope::new(), "/")
            .map(|_| ())
    }

    fn with_wrapper(panel: &str) -> Result<(), Failure> {
        Self::render(
            panel,
            ("client/services/OrderListService.js", Self::WRAPPER),
        )
    }

    fn without_wrapper(panel: &str) -> Result<(), Failure> {
        Self::render(panel, ("client/services/ShellService.js", Self::PLAIN))
    }
}

#[test]
fn a_view_that_calls_a_remote_is_refused_and_names_the_service_that_wraps_it() {
    let text = RemoteViews::with_wrapper("<button on:click=\"ordersClient.list()\">Load</button>")
        .expect_err("a view may not name a remote")
        .to_string();
    assert!(text.contains("TY1377"), "{text}");
    assert!(
        text.contains("The view 'client/components/orders/panel/tac.html' names the remote `ordersClient` (OrdersClient in client/remotes). A view reaches only client services, never a remote."),
        "{text}"
    );
    assert!(
        text.contains("OrderListService could wrap OrdersClient"),
        "{text}"
    );
}

#[test]
fn a_view_that_reads_a_remote_is_refused() {
    let text = RemoteViews::with_wrapper("<p>{ordersClient.count}</p>")
        .expect_err("a view may not read a remote")
        .to_string();
    assert!(text.contains("TY1377"), "{text}");
}

#[test]
fn without_a_wrapping_service_the_help_suggests_a_new_service() {
    let text = RemoteViews::without_wrapper("<p>{ordersClient.count}</p>")
        .expect_err("a view may not read a remote")
        .to_string();
    assert!(
        text.contains("a new OrdersService could wrap OrdersClient"),
        "{text}"
    );
}

#[test]
fn a_view_that_calls_the_wrapping_service_is_accepted() {
    let rendered = RemoteViews::with_wrapper("<button on:click=\"orderList.load()\">Load</button>");
    assert!(rendered.is_ok(), "{rendered:?}");
}

#[test]
fn a_view_with_an_unrelated_name_is_accepted() {
    let rendered = RemoteViews::without_wrapper("<p>{shell.title} {orders.count}</p>");
    assert!(rendered.is_ok(), "{rendered:?}");
}
