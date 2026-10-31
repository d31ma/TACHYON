#![allow(clippy::expect_used)]

use super::fsharp_tests::fsharp_web_ready;
use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use serde_json::json;
use std::path::{Path, PathBuf};

struct DotnetMarkerPage;

impl DotnetMarkerPage {
    const ORDER_MODEL: (&'static str, &'static str) = (
        "client/models/OrderPlaced.cs",
        "public record OrderPlaced(int Total);\n",
    );

    const AUDIT: (&'static str, &'static str) = (
        "client/services/AuditService.cs",
        "using System.Collections.Generic;\nusing Models;\n\n\
         [Service]\npublic class AuditService\n{\n    public List<string> Heard = new List<string>();\n    \
         public ApplicationEventPublisher Publisher = new ApplicationEventPublisher();\n\n    \
         public int Place(int total)\n    {\n        Publisher.PublishEvent(new OrderPlaced(total));\n        return Heard.Count;\n    }\n\n    \
         [OnEvent]\n    [InBackground]\n    [Order(-100)]\n    public void Archive(OrderPlaced order)\n    {\n        Heard.Add(\"background \" + order.Total);\n    }\n\n    \
         [OnEvent]\n    [Order(10)]\n    public void Last(OrderPlaced order)\n    {\n        Heard.Add(\"last \" + order.Total);\n    }\n\n    \
         [OnEvent]\n    public void Middle(OrderPlaced order)\n    {\n        Heard.Add(\"middle \" + order.Total);\n    }\n\n    \
         [OnEvent]\n    [Order(-5)]\n    public void First(OrderPlaced order)\n    {\n        Heard.Add(\"first \" + order.Total);\n    }\n}\n",
    );

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
         const listeners = Object.fromEntries(plan.types[0].listeners.map((listener) => [listener.key.replace(/^__event_\\d+_/, '').replace(/^./, (first) => first.toLowerCase()), [listener.order, listener.asynchronous === true]]))\n\
         console.log(JSON.stringify({ during, afterCall, settled: audit.heard, seen, listeners }))\n\
         process.exit(0)\n";

    const CSHARP_SHELF: (&'static str, &'static str) = (
        "client/services/ShelfService.cs",
        "[Service]\n[Lifetime({\"value\":\"prototype\"})]\npublic class ShelfService\n{\n    \
         [LocalState]\n    public int Visits = 0;\n    [SessionState]\n    public string Draft = \"\";\n\n    \
         public int Visit()\n    {\n        Visits += 1;\n        return Visits;\n    }\n}\n",
    );

    const FSHARP_SHELF: (&'static str, &'static str) = (
        "client/services/ShelfService.fs",
        "[<Service>]\n[<Lifetime({\"value\":\"prototype\"})>]\ntype ShelfService() =\n    \
         [<LocalState>]\n    member val Visits = 0 with get, set\n    [<SessionState>]\n    member val Draft = \"\" with get, set\n\n    \
         member this.Visit() : int =\n        this.Visits <- this.Visits + 1\n        this.Visits\n",
    );

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

    const PAGE_FETCH: &'static str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         globalThis.location = { href: 'http://127.0.0.1:4000/page' }\n";

    fn ready() -> bool {
        let ready = fsharp_web_ready();
        assert!(
            ready || std::env::var_os("CI").is_none(),
            "CI must provide the .NET wasm-tools workload and node"
        );
        if !ready {
            eprintln!("skipping: the .NET wasm-tools workload or node is unavailable");
        }
        ready
    }

    fn write(root: &Path, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(&path, bytes).expect("file");
        path
    }

    fn sources(root: &Path, files: &[(&str, &str)]) -> PathBuf {
        let mut service = PathBuf::new();
        for (relative, text) in files {
            service = Self::write(root, relative, text.as_bytes());
        }
        service
    }

    async fn publish(root: &Path, files: &[(&str, &str)], (instance, class): (&str, &str)) {
        let path = Self::sources(root, files);
        let services = [(
            String::from(instance),
            String::from(class),
            WebLanguage::CSharp,
            path,
        )];
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Development,
        )
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

    fn page_storage(root: &Path, files: &[(&str, &str)], instance: &str) -> serde_json::Value {
        Self::sources(root, files);
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
            format!("{}{driver}", Self::PAGE_FETCH).as_bytes(),
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

    fn assert_restored(report: &serde_json::Value) {
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
}

#[tokio::test]
async fn a_csharp_web_service_runs_ordered_and_background_listeners_as_webassembly() {
    if !DotnetMarkerPage::ready() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    DotnetMarkerPage::publish(
        root.path(),
        &[DotnetMarkerPage::ORDER_MODEL, DotnetMarkerPage::AUDIT],
        ("audit", "AuditService"),
    )
    .await;
    let report = DotnetMarkerPage::drive(root.path(), DotnetMarkerPage::AUDIT_DRIVER);
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
async fn a_csharp_session_state_field_survives_a_reload() {
    let root = tempfile::tempdir().expect("project");
    let files = [DotnetMarkerPage::CSHARP_SHELF];
    let stored = DotnetMarkerPage::page_storage(root.path(), &files, "shelf");
    assert_eq!(stored, json!({"draft": "session", "visits": "local"}));
    if !DotnetMarkerPage::ready() {
        return;
    }
    DotnetMarkerPage::publish(root.path(), &files, ("shelf", "ShelfService")).await;
    let report = DotnetMarkerPage::drive(root.path(), DotnetMarkerPage::SHELF_DRIVER);
    DotnetMarkerPage::assert_restored(&report);
}

#[tokio::test]
async fn an_fsharp_session_state_field_survives_a_reload() {
    let root = tempfile::tempdir().expect("project");
    let files = [DotnetMarkerPage::FSHARP_SHELF];
    let stored = DotnetMarkerPage::page_storage(root.path(), &files, "shelf");
    assert_eq!(stored, json!({"draft": "session", "visits": "local"}));
    if !DotnetMarkerPage::ready() {
        return;
    }
    DotnetMarkerPage::publish(root.path(), &files, ("shelf", "ShelfService")).await;
    let report = DotnetMarkerPage::drive(root.path(), DotnetMarkerPage::SHELF_DRIVER);
    DotnetMarkerPage::assert_restored(&report);
}
