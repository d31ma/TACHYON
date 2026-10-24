#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

struct CppMarkerPage;

impl CppMarkerPage {
    const TOPIC_MODEL: (&'static str, &'static str) = (
        "client/models/InventoryRecord.hpp",
        "#pragma once\n#include <string>\n\nstruct [[Topic]] InventoryRecord {\n    std::string sku;\n    int level = 0;\n};\n",
    );

    const INVENTORY: &'static str = "#pragma once\n#include <string>\n#include <vector>\n#include \"../models/InventoryRecord.hpp\"\n\n\
         class [[Service]] InventoryService {\npublic:\n    std::vector<std::string> skus;\n    int level = 0;\n\n    \
         [[OnEvent]] void add(const InventoryRecord& record) { skus.push_back(record.sku); level += record.level; }\n};\n";

    const INVENTORY_DRIVER: &'static str = "globalThis.location = { href: 'http://127.0.0.1:4000/page' }\n\
         const sources = []\n\
         globalThis.EventSource = class { constructor(url) { this.url = String(url); this.closed = false; sources.push(this) } addEventListener() {} close() { this.closed = true } }\n\
         const { default: InventoryService, __tachyon_event_plan__: plan } = await import('./InventoryService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const inventory = new InventoryService({})\n\
         bus.attach(inventory, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(inventory)\n\
         const before = inventory.skus\n\
         for (const source of sources) source.onmessage({ data: JSON.stringify({ sku: 'anvil', level: 3 }), lastEventId: '1' })\n\
         for (const source of sources) source.onmessage({ data: JSON.stringify({ sku: 'rope', level: 7 }), lastEventId: '2' })\n\
         const report = { topics: sources.map((source) => source.url), before, skus: inventory.skus, level: inventory.level }\n\
         bus.dispose(inventory)\n\
         await new Promise((resolve) => setTimeout(resolve, 50))\n\
         report.closed = sources.map((source) => source.closed)\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";

    const ORDER_MODEL: (&'static str, &'static str) = (
        "client/models/OrderPlaced.hpp",
        "#pragma once\n\nstruct OrderPlaced {\n    int total = 0;\n};\n",
    );

    const AUDIT: &'static str = "#pragma once\n#include <string>\n#include <vector>\n#include \"../models/OrderPlaced.hpp\"\n\n\
         class [[Service]] AuditService {\npublic:\n    std::vector<std::string> heard;\n    ApplicationEventPublisher applicationEventPublisher;\n\n    \
         int place(int total) {\n        applicationEventPublisher.publishEvent(OrderPlaced{total});\n        return static_cast<int>(heard.size());\n    }\n\n    \
         [[OnEvent]] [[InBackground]] [[Order(-100)]]\n    void archive(const OrderPlaced& order) { heard.push_back(\"background \" + std::to_string(order.total)); }\n\n    \
         [[OnEvent]] [[Order(10)]]\n    void last(const OrderPlaced& order) { heard.push_back(\"last \" + std::to_string(order.total)); }\n\n    \
         [[OnEvent]]\n    void middle(const OrderPlaced& order) { heard.push_back(\"middle \" + std::to_string(order.total)); }\n\n    \
         [[OnEvent]] [[Order(-5)]]\n    void first(const OrderPlaced& order) { heard.push_back(\"first \" + std::to_string(order.total)); }\n};\n";

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
         const listeners = Object.fromEntries(plan.types[0].listeners.map((listener) => [listener.key, [listener.order, listener.asynchronous]]))\n\
         console.log(JSON.stringify({ during, afterCall, settled: audit.heard, seen, listeners }))\n\
         process.exit(0)\n";

    const SHELF: &'static str = "#pragma once\n#include <string>\n\n\
         class [[Service, Lifetime({\"value\":\"prototype\"})]] ShelfService {\npublic:\n    [[LocalState]] int visits = 0;\n    [[SessionState]] std::string draft;\n\n    \
         int visit() {\n        visits += 1;\n        return visits;\n    }\n};\n";

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

    const FILE_FETCH: &'static str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n";

    fn tools_available() -> bool {
        let found = ["em++", "node"].iter().all(|program| {
            Command::new(program)
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success())
        });
        if !found {
            eprintln!("skipping: em++ (Emscripten) or node is unavailable");
        }
        found
    }

    fn write(root: &Path, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(&path, bytes).expect("file");
        path
    }

    async fn publish(root: &Path, models: &[(&str, &str)], service: (&str, &str, &str)) {
        for (relative, text) in models {
            Self::write(root, relative, text.as_bytes());
        }
        let (instance, class, text) = service;
        let path = Self::write(
            root,
            &format!("client/services/{class}.hpp"),
            text.as_bytes(),
        );
        let services = [(
            String::from(instance),
            String::from(class),
            WebLanguage::Cpp,
            path,
        )];
        let profile = crate::BuildProfile::Development;
        let built = build_all(root, &services, &BuildJobs::new(1), profile)
            .await
            .unwrap_or_else(|failure| panic!("{failure}"));
        for (name, bytes) in built.files {
            Self::write(root, &name, &bytes);
        }
        Self::write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            )
            .as_bytes(),
        );
        Self::write(
            root,
            "tac-client.js",
            include_str!("../template/tac-client.js").as_bytes(),
        );
    }

    fn page_storage(root: &Path, instance: &str) -> serde_json::Value {
        let registry = crate::template::ServiceRegistry::discover(root)
            .unwrap_or_else(|failure| panic!("{failure}"));
        let state = json!(registry.get(instance).expect("service").stored());
        Self::write(root, "state.json", state.to_string().as_bytes());
        state
    }

    fn drive(root: &Path, driver: &str) -> serde_json::Value {
        Self::write(
            root,
            "drive.mjs",
            format!("{}{driver}", Self::FILE_FETCH).as_bytes(),
        );
        let output = NodeDriver::output(NodeDriver::command().arg("drive.mjs").current_dir(root))
            .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let last = stdout.lines().last().unwrap_or_default();
        serde_json::from_str(last).expect("report")
    }
}

#[tokio::test]
async fn a_cpp_topic_model_reaches_its_listener_from_its_topic_stream_on_the_page() {
    if !CppMarkerPage::tools_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    CppMarkerPage::publish(
        root.path(),
        &[CppMarkerPage::TOPIC_MODEL],
        ("inventory", "InventoryService", CppMarkerPage::INVENTORY),
    )
    .await;
    let report = CppMarkerPage::drive(root.path(), CppMarkerPage::INVENTORY_DRIVER);
    assert_eq!(
        report,
        json!({
            "topics": ["/.tachyon/topics/inventory-record"],
            "before": [],
            "skus": ["anvil", "rope"],
            "level": 10,
            "closed": [true],
        }),
        "{report}"
    );
}

#[tokio::test]
async fn cpp_listeners_run_in_their_order_and_a_background_listener_after_the_delivery_pass() {
    if !CppMarkerPage::tools_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    CppMarkerPage::publish(
        root.path(),
        &[CppMarkerPage::ORDER_MODEL],
        ("audit", "AuditService", CppMarkerPage::AUDIT),
    )
    .await;
    let report = CppMarkerPage::drive(root.path(), CppMarkerPage::AUDIT_DRIVER);
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
}

#[tokio::test]
async fn a_cpp_local_state_field_is_written_to_page_storage_and_restored_after_a_reload() {
    if !CppMarkerPage::tools_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    CppMarkerPage::publish(
        root.path(),
        &[],
        ("shelf", "ShelfService", CppMarkerPage::SHELF),
    )
    .await;
    let stored = CppMarkerPage::page_storage(root.path(), "shelf");
    assert_eq!(stored, json!({"draft": "session", "visits": "local"}));
    let report = CppMarkerPage::drive(root.path(), CppMarkerPage::SHELF_DRIVER);
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
