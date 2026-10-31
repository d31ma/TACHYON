#![allow(clippy::expect_used)]

use super::swift_tests::{publish, run_driver, swift_web_ready, write};
use serde_json::json;
use std::path::Path;

struct SwiftMarkerPage;

impl SwiftMarkerPage {
    const ORDER_MODEL: (&'static str, &'static str) = (
        "client/models/OrderPlaced.swift",
        "struct OrderPlaced {\n    let total: Int\n}\n",
    );

    const AUDIT: (&'static str, &'static str) = (
        "client/services/AuditService.swift",
        r#"@Service
final class AuditService {
    var heard: [String] = []
    let applicationEventPublisher: ApplicationEventPublisher

    func place(_ total: Int) -> Int {
        applicationEventPublisher.publishEvent(OrderPlaced(total: total))
        return heard.count
    }

    @OnEvent
    @InBackground
    @Order(-100)
    func archive(_ order: OrderPlaced) {
        heard.append("background \(order.total)")
    }

    @OnEvent
    @Order(10)
    func last(_ order: OrderPlaced) {
        heard.append("last \(order.total)")
    }

    @OnEvent
    func middle(_ order: OrderPlaced) {
        heard.append("middle \(order.total)")
    }

    @OnEvent
    @Order(-5)
    func first(_ order: OrderPlaced) {
        heard.append("first \(order.total)")
    }

    @OnStop
    func stop() {
        print("hook: stop audit after \(heard.count)")
    }
}
"#,
    );

    const AUDIT_DRIVER: &'static str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const lines = []\n\
         for (const level of ['log', 'info', 'warn', 'error']) console[level] = (...values) => lines.push(values.join(' '))\n\
         const { default: AuditService, __tachyon_event_plan__: plan } = await import('./AuditService.js')\n\
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
         const settled = audit.heard\n\
         const listeners = Object.fromEntries(plan.types[0].listeners.map((listener) => [listener.key.replace(/^__event_\\d+_/, ''), [listener.order, listener.asynchronous]]))\n\
         const running = lines.filter((line) => line.startsWith('hook: '))\n\
         bus.dispose(audit)\n\
         await new Promise((resolve) => setTimeout(resolve, 50))\n\
         const stopped = lines.filter((line) => line.startsWith('hook: '))\n\
         process.stdout.write(`${JSON.stringify({ during, afterCall, settled, seen, listeners, running, stopped })}\\n`)\n\
         process.exit(0)\n";

    const SHELF: (&'static str, &'static str) = (
        "client/services/ShelfService.swift",
        "@Service\n@Lifetime({\"value\":\"prototype\"})\nfinal class ShelfService {\n    \
         @LocalState\n    var visits: Int = 0\n    @SessionState\n    var draft: String = \"\"\n\n    \
         func visit() -> Int {\n        visits += 1\n        return visits\n    }\n}\n",
    );

    const SHELF_DRIVER: &'static str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const client = await readFile('./tac-client.js', 'utf8')\n\
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

    fn with_page_runtime(root: &Path) {
        write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        );
        write(
            root,
            "tac-client.js",
            include_str!("../template/tac-client.js"),
        );
    }

    fn page_storage(root: &Path, instance: &str) -> serde_json::Value {
        let registry = crate::template::ServiceRegistry::discover(root)
            .unwrap_or_else(|failure| panic!("{failure}"));
        let state = json!(registry.get(instance).expect("service").stored());
        write(root, "state.json", &state.to_string());
        state
    }
}

#[tokio::test]
async fn a_swift_web_service_runs_its_stop_hook_ordered_and_background_listeners() {
    if !swift_web_ready() {
        eprintln!("skipping: the Swift SDK for WebAssembly, swiftc or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let (model, model_text) = SwiftMarkerPage::ORDER_MODEL;
    write(root.path(), model, model_text);
    let (service, service_text) = SwiftMarkerPage::AUDIT;
    write(root.path(), service, service_text);
    publish(root.path(), "audit", "AuditService").await;
    SwiftMarkerPage::with_page_runtime(root.path());
    let report = run_driver(root.path(), SwiftMarkerPage::AUDIT_DRIVER);
    assert_eq!(
        report["listeners"],
        json!({"archive": [-100, true], "last": [10, false], "middle": [0, false], "first": [-5, false]}),
        "{report}"
    );
    assert_eq!(report["during"], 0, "{report}");
    assert_eq!(
        report["seen"],
        json!([[3, ["first 3", "middle 3"]]]),
        "{report}"
    );
    assert_eq!(
        report["afterCall"],
        json!(["first 3", "middle 3", "last 3"]),
        "{report}"
    );
    assert_eq!(
        report["settled"],
        json!(["first 3", "middle 3", "last 3", "background 3"]),
        "{report}"
    );
    assert_eq!(report["running"], json!([]), "{report}");
    assert_eq!(
        report["stopped"],
        json!(["hook: stop audit after 4"]),
        "{report}"
    );
}

#[tokio::test]
async fn a_swift_web_service_keeps_local_and_session_fields_across_a_reload() {
    if !swift_web_ready() {
        eprintln!("skipping: the Swift SDK for WebAssembly, swiftc or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let (service, service_text) = SwiftMarkerPage::SHELF;
    write(root.path(), service, service_text);
    publish(root.path(), "shelf", "ShelfService").await;
    SwiftMarkerPage::with_page_runtime(root.path());
    let stored = SwiftMarkerPage::page_storage(root.path(), "shelf");
    assert_eq!(stored, json!({"draft": "session", "visits": "local"}));
    let report = run_driver(root.path(), SwiftMarkerPage::SHELF_DRIVER);
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
