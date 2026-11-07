#![allow(clippy::expect_used)]

use super::helper_tests::HelperPage;
use serde_json::json;
use std::path::Path;

struct MarkerPage;

impl MarkerPage {
    const CONVERTER_FILES: [(&'static str, &'static str); 5] = [
        (
            "client/models/Detail.lua",
            "---@class Detail\n---@field name string\n---@field secret string\nDetail = {}\n",
        ),
        (
            "client/models/Product.lua",
            "---@class Product\n---@field id integer\n---@field name string\n---@field detail Detail\n---@field items Detail[]\n---@field secret string\nProduct = {}\n",
        ),
        (
            "client/models/ShortDetail.lua",
            "---@class ShortDetail\n---@field name string\nShortDetail = {}\n",
        ),
        (
            "client/models/Summary.lua",
            "---@class Summary\n---@field id number\n---@field name string\n---@field detail ShortDetail\n---@field items ShortDetail[]\n---@field kind string\nSummary = {}\n",
        ),
        (
            "client/converters/ProductConverter.lua",
            "-- @Converter\n---@class ProductConverter\nProductConverter = {}\n\n\
             ---@param product Product\n---@return Summary\n-- @Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})\nfunction ProductConverter:map(product)\nend\n\n\
             ---@param product Product\n---@return Product\nfunction ProductConverter:copy(product)\nend\n\n\
             ---@param product table\n---@return table\n-- @Convert({\"fields\":[{\"source\":\"detail.name\",\"target\":\"detailName\"},{\"source\":\"name\",\"target\":\"title\"},{\"target\":\"kind\",\"constant\":\"card\"}]})\nfunction ProductConverter:to_card(product)\nend\n",
        ),
    ];

    const CATALOG: &'static str = "-- @Service\n---@class CatalogService\nCatalogService = {\n  -- @Needs\n  ---@type ProductConverter\n  product_converter = nil,\n}\n\n\
         ---@return table\nfunction CatalogService:summary()\n  local detail = setmetatable({ name = \"detail\", secret = \"detail-private\" }, Detail)\n  local product = setmetatable({ id = 7, name = \"Anvil\", detail = detail, items = { detail }, secret = \"product-private\" }, Product)\n  return self.product_converter:map(self.product_converter:copy(product))\nend\n\n\
         ---@return table\nfunction CatalogService:card()\n  return self.product_converter:to_card({ id = 7, name = \"Anvil\", detail = { name = \"steel\" }, secret = \"hidden\" })\nend\n";

    const CATALOG_DRIVER: &'static str = "const { default: CatalogService } = await import('./CatalogService.js')\n\
         const catalog = new CatalogService({})\n\
         console.log(JSON.stringify({ summary: catalog.summary(), card: catalog.card(), fields: Object.keys(catalog) }))\n\
         process.exit(0)\n";

    const SHELF: &'static str = "-- @Service\n---@class ShelfService\nShelfService = {\n  -- @LocalState\n  ---@type integer\n  visits = 0,\n  -- @SessionState\n  ---@type string\n  draft = \"\",\n}\n\n\
         ---@return integer\nfunction ShelfService:visit()\n  self.visits = self.visits + 1\n  return self.visits\nend\n";

    const SHELF_DRIVER: &'static str = "const client = await readFile('./tac-client.js', 'utf8')\n\
         const host = client.slice(client.indexOf('const webStorage = '), client.indexOf('const persistScope = '))\n\
         class MemoryStorage {\n\
         \x20 constructor() { this.items = new Map() }\n\
         \x20 getItem(key) { return this.items.has(key) ? this.items.get(key) : null }\n\
         \x20 setItem(key, value) { this.items.set(key, String(value)) }\n\
         \x20 removeItem(key) { this.items.delete(key) }\n\
         }\n\
         const { default: ShelfService } = await import('./ShelfService.js')\n\
         const stored = JSON.parse(await readFile('./state.json', 'utf8'))\n\
         const local = new MemoryStorage()\n\
         const open = (session) => {\n\
         \x20 const { Owners, bindPersistentFields } = new Function('localStorage', 'sessionStorage', `${host}\\nreturn { Owners, bindPersistentFields }`)(local, session)\n\
         \x20 const shelf = new ShelfService({})\n\
         \x20 Owners.stored.set(shelf, stored)\n\
         \x20 bindPersistentFields(shelf, 'service:shelf:/')\n\
         \x20 return shelf\n\
         }\n\
         const tab = new MemoryStorage()\n\
         const page = open(tab)\n\
         const visited = [page.visit(), page.visit()]\n\
         page.draft = 'chapter one'\n\
         page.__tachyonDispose()\n\
         const reloaded = open(tab)\n\
         const afterReload = { visits: reloaded.visits, draft: reloaded.draft, visit: reloaded.visit() }\n\
         reloaded.__tachyonDispose()\n\
         const other = open(new MemoryStorage())\n\
         const newTab = { visits: other.visits, draft: other.draft }\n\
         other.__tachyonDispose()\n\
         const unbound = new ShelfService({})\n\
         const fresh = unbound.visits\n\
         unbound.__tachyonDispose()\n\
         console.log(JSON.stringify({ fresh, visited, afterReload, newTab, saved: Object.fromEntries(local.items), session: Object.fromEntries(tab.items) }))\n\
         process.exit(0)\n";

    const AUDIT_MODEL: (&'static str, &'static str) = (
        "client/models/OrderPlaced.lua",
        "---@class OrderPlaced\n---@field total integer\nOrderPlaced = {}\n",
    );

    const AUDIT: &'static str = "-- @Service\n---@class AuditService\nAuditService = {\n  ---@type string[]\n  heard = {},\n  ---@type ApplicationEventPublisher\n  application_event_publisher = nil,\n}\n\n\
         ---@param total integer\n---@return integer\nfunction AuditService:place(total)\n  self.application_event_publisher:publish_event(setmetatable({ total = total }, OrderPlaced))\n  return #self.heard\nend\n\n\
         ---@param order OrderPlaced\n---@return nil\n-- @OnEvent\n-- @InBackground\nfunction AuditService:archive(order)\n  self.heard[#self.heard + 1] = \"background \" .. order.total\nend\n\n\
         ---@param order OrderPlaced\n---@return nil\n-- @OnEvent\nfunction AuditService:record(order)\n  self.heard[#self.heard + 1] = \"now \" .. order.total\nend\n";

    const AUDIT_DRIVER: &'static str = "const { default: AuditService, __tachyon_event_plan__: plan } = await import('./AuditService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const audit = new AuditService({})\n\
         const seen = []\n\
         bus.attach(audit, plan.types[0], { applicationEventPublisher: null })\n\
         bus.attach({ receive: (value) => seen.push([value.total, audit.heard]) }, { listeners: [{ method: 'receive', event: 'OrderPlaced', order: 5 }] }, {})\n\
         await bus.initialize(audit)\n\
         const during = audit.place(3)\n\
         const afterCall = audit.heard\n\
         await new Promise((resolve) => setTimeout(resolve, 20))\n\
         const background = plan.types[0].listeners.filter((listener) => listener.asynchronous === true).map((listener) => listener.key)\n\
         console.log(JSON.stringify({ during, afterCall, settled: audit.heard, seen, background }))\n\
         process.exit(0)\n";

    fn write_all(root: &Path, files: &[(&str, &str)]) {
        for (relative, text) in files {
            HelperPage::write(root, relative, text);
        }
    }

    fn event_runtime(root: &Path) {
        HelperPage::write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        );
    }

    fn page_storage(root: &Path, instance: &str) -> serde_json::Value {
        HelperPage::write(
            root,
            "tac-client.js",
            include_str!("../template/tac-client.js"),
        );
        let registry = crate::template::ServiceRegistry::discover(root)
            .unwrap_or_else(|failure| panic!("{failure}"));
        let stored = registry.get(instance).expect("service").stored();
        let state = json!(stored);
        HelperPage::write(root, "state.json", &state.to_string());
        state
    }
}

#[tokio::test]
async fn a_lua_service_maps_through_the_converter_it_needs_on_the_page() {
    if let Some(reason) = HelperPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    MarkerPage::write_all(root.path(), &MarkerPage::CONVERTER_FILES);
    HelperPage::publish(
        root.path(),
        "catalog",
        "CatalogService",
        MarkerPage::CATALOG,
    )
    .await;
    let report = HelperPage::run(root.path(), MarkerPage::CATALOG_DRIVER);
    assert_eq!(
        report["summary"],
        json!({"id": 7, "name": "Anvil", "detail": {"name": "detail"}, "items": [{"name": "detail"}], "kind": "product"}),
        "{report}"
    );
    assert_eq!(
        report["card"],
        json!({"detailName": "steel", "title": "Anvil", "kind": "card"}),
        "{report}"
    );
    assert_eq!(report["fields"], json!([]), "{report}");
}

#[tokio::test]
async fn a_lua_local_state_field_is_written_to_page_storage_and_restored_after_a_reload() {
    if let Some(reason) = HelperPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    HelperPage::publish(root.path(), "shelf", "ShelfService", MarkerPage::SHELF).await;
    let stored = MarkerPage::page_storage(root.path(), "shelf");
    assert_eq!(stored, json!({"draft": "session", "visits": "local"}));
    let report = HelperPage::run(root.path(), MarkerPage::SHELF_DRIVER);
    assert_eq!(report["fresh"], 0, "{report}");
    assert_eq!(report["visited"], json!([1, 2]), "{report}");
    assert_eq!(
        report["afterReload"],
        json!({"visits": 2, "draft": "chapter one", "visit": 3}),
        "{report}"
    );
    assert_eq!(
        report["newTab"],
        json!({"visits": 3, "draft": ""}),
        "{report}"
    );
    assert_eq!(
        report["saved"],
        json!({"tac:service:shelf:/:visits": "3"}),
        "{report}"
    );
    assert_eq!(
        report["session"],
        json!({"tac:service:shelf:/:draft": "\"chapter one\""}),
        "{report}"
    );
}

#[tokio::test]
async fn a_lua_background_listener_runs_after_the_delivery_pass_on_the_page() {
    if let Some(reason) = HelperPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    MarkerPage::write_all(root.path(), &[MarkerPage::AUDIT_MODEL]);
    HelperPage::publish(root.path(), "audit", "AuditService", MarkerPage::AUDIT).await;
    MarkerPage::event_runtime(root.path());
    let report = HelperPage::run(root.path(), MarkerPage::AUDIT_DRIVER);
    assert_eq!(report["background"], json!(["archive"]), "{report}");
    assert_eq!(report["during"], 0, "{report}");
    assert_eq!(report["afterCall"], json!(["now 3"]), "{report}");
    assert_eq!(report["seen"], json!([[3, ["now 3"]]]), "{report}");
    assert_eq!(
        report["settled"],
        json!(["now 3", "background 3"]),
        "{report}"
    );
}
