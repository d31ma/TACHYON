#![allow(clippy::expect_used)]

use super::kotlin_tests::kotlin_web_ready;
use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use serde_json::json;
use std::path::Path;

struct KotlinMarkerProofs;

impl KotlinMarkerProofs {
    const BACKGROUND_FILES: [(&'static str, &'static str); 2] = [
        (
            "client/models/OrderPlaced.kt",
            "data class OrderPlaced(val total: Long)\n",
        ),
        (
            "client/services/JournalService.kt",
            "import models.OrderPlaced\n\n@Service\nclass JournalService {\n    \
             val applicationEventPublisher: ApplicationEventPublisher\n    \
             var heard: List<String> = listOf()\n\n    \
             fun place(total: Long) {\n        heard = heard + \"place $total\"\n        \
             applicationEventPublisher.publishEvent(OrderPlaced(total))\n        heard = heard + \"placed $total\"\n    }\n\n    \
             @OnEvent\n    @InBackground\n    fun audit(order: OrderPlaced) {\n        heard = heard + \"audit ${order.total}\"\n    }\n\n    \
             @OnEvent\n    fun record(order: OrderPlaced) {\n        heard = heard + \"record ${order.total}\"\n    }\n}\n",
        ),
    ];

    const BACKGROUND_DRIVER: &'static str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const { default: JournalService, __tachyon_event_plan__: plan } = await import('./JournalService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         let failures = 0\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { failures += 1 } })\n\
         const journal = new JournalService({})\n\
         bus.attach(journal, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(journal)\n\
         journal.place(3)\n\
         const returned = [...journal.heard]\n\
         await new Promise((settle) => setTimeout(settle, 20))\n\
         const after = [...journal.heard]\n\
         const listeners = plan.types[0].listeners.map(({ method, asynchronous }) => [method.split('_').pop(), asynchronous === true])\n\
         console.log(JSON.stringify({ returned, after, listeners, failures }))\n\
         process.exit(0)\n";

    const STOP_FILES: [(&'static str, &'static str); 1] = [(
        "client/services/WelcomeService.kt",
        "@Service\nclass WelcomeService {\n    var count: Int = 0\n\n    \
         @OnStart\n    fun start() {\n        println(\"hook: start welcome\")\n    }\n\n    \
         @OnStop\n    fun stop() {\n        println(\"hook: stop welcome at $count\")\n    }\n\n    \
         fun bump(): Int {\n        count += 1\n        return count\n    }\n}\n",
    )];

    const STOP_DRIVER: &'static str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const lines = []\n\
         for (const level of ['log', 'info', 'warn', 'error']) console[level] = (...values) => lines.push(values.join(' '))\n\
         const { default: WelcomeService, __tachyon_event_plan__: plan } = await import('./WelcomeService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const welcome = new WelcomeService({})\n\
         bus.attach(welcome, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(welcome)\n\
         const bumped = [welcome.bump(), welcome.bump()]\n\
         const running = lines.filter((line) => line.startsWith('hook: '))\n\
         bus.dispose(welcome)\n\
         await new Promise((resolve) => setTimeout(resolve, 50))\n\
         process.stdout.write(`${JSON.stringify({ bumped, running, lines: lines.filter((line) => line.startsWith('hook: ')) })}\\n`)\n\
         process.exit(0)\n";

    const STATE_SOURCE: (&'static str, &'static str) = (
        "client/services/DraftService.kt",
        "@Service\nclass DraftService {\n    @LocalState\n    var visits: Int = 0\n    @SessionState\n    var draft: String = \"\"\n\n    \
         fun visit(): Int {\n        visits += 1\n        return visits\n    }\n\n    fun write(text: String) {\n        draft = text\n    }\n}\n",
    );

    const STATE_DRIVER: &'static str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const client = await readFile('./tac-client.js', 'utf8')\n\
         const host = client.slice(client.indexOf('const webStorage = '), client.indexOf('const persistScope = '))\n\
         class MemoryStorage {\n\
         \x20 constructor() { this.items = new Map() }\n\
         \x20 getItem(key) { return this.items.has(key) ? this.items.get(key) : null }\n\
         \x20 setItem(key, value) { this.items.set(key, String(value)) }\n\
         \x20 removeItem(key) { this.items.delete(key) }\n\
         }\n\
         const local = new MemoryStorage()\n\
         const { default: DraftService } = await import('./DraftService.js')\n\
         const stored = JSON.parse(await readFile('./state.json', 'utf8'))\n\
         const open = (session) => {\n\
         \x20 const { Owners, bindPersistentFields } = new Function('localStorage', 'sessionStorage', `${host}\\nreturn { Owners, bindPersistentFields }`)(local, session)\n\
         \x20 const draft = new DraftService({})\n\
         \x20 Owners.stored.set(draft, stored)\n\
         \x20 bindPersistentFields(draft, 'service:draft:/')\n\
         \x20 draft.__tachyonStarted()\n\
         \x20 return draft\n\
         }\n\
         const tab = new MemoryStorage()\n\
         const page = open(tab)\n\
         page.visit(); page.visit()\n\
         page.write('chapter one')\n\
         page.__tachyonDispose()\n\
         const reloaded = open(tab)\n\
         const afterReload = { visits: reloaded.visits, draft: reloaded.draft, visit: reloaded.visit() }\n\
         reloaded.__tachyonDispose()\n\
         const other = open(new MemoryStorage())\n\
         const newTab = { visits: other.visits, draft: other.draft }\n\
         other.__tachyonDispose()\n\
         console.log(JSON.stringify({ afterReload, newTab, saved: Object.fromEntries(local.items), session: Object.fromEntries(tab.items) }))\n\
         process.exit(0)\n";

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(&path, text).expect("file");
    }

    async fn publish(root: &Path, files: &[(&str, &str)], instance: &str, class: &str) {
        for (relative, text) in files {
            Self::write(root, relative, text);
        }
        let services = vec![(
            String::from(instance),
            String::from(class),
            WebLanguage::Kotlin,
            root.join(format!("client/services/{class}.kt")),
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
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            std::fs::write(path, bytes).expect("publish");
        }
        Self::write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        );
        Self::write(
            root,
            "tac-client.js",
            include_str!("../template/tac-client.js"),
        );
    }

    fn drive(root: &Path, driver: &str) -> serde_json::Value {
        std::fs::write(root.join("drive.mjs"), driver).expect("driver");
        let output = NodeDriver::output(NodeDriver::command().arg("drive.mjs").current_dir(root))
            .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.trim()).expect("report")
    }

    fn stored_fields() -> serde_json::Value {
        let (relative, source) = Self::STATE_SOURCE;
        let fields = crate::behavior::StoredState::fields(Path::new(relative), source)
            .unwrap_or_else(|failure| panic!("{failure}"));
        json!(fields)
    }
}

#[tokio::test]
async fn a_kotlin_in_background_listener_runs_after_the_publishing_call_returns() {
    if !kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, TAC_KOTLIN_WASM_STDLIB or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    KotlinMarkerProofs::publish(
        root.path(),
        &KotlinMarkerProofs::BACKGROUND_FILES,
        "journal",
        "JournalService",
    )
    .await;
    let report = KotlinMarkerProofs::drive(root.path(), KotlinMarkerProofs::BACKGROUND_DRIVER);
    assert_eq!(
        report["listeners"],
        json!([["audit", true], ["record", false]]),
        "{report}"
    );
    assert_eq!(
        report["returned"],
        json!(["place 3", "placed 3", "record 3"]),
        "{report}"
    );
    assert_eq!(
        report["after"],
        json!(["place 3", "placed 3", "record 3", "audit 3"]),
        "{report}"
    );
    assert_eq!(report["failures"], 0, "{report}");
}

#[tokio::test]
async fn a_kotlin_service_runs_its_stop_hook_in_the_page_when_its_owner_is_disposed() {
    if !kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, TAC_KOTLIN_WASM_STDLIB or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    KotlinMarkerProofs::publish(
        root.path(),
        &KotlinMarkerProofs::STOP_FILES,
        "welcome",
        "WelcomeService",
    )
    .await;
    let report = KotlinMarkerProofs::drive(root.path(), KotlinMarkerProofs::STOP_DRIVER);
    assert_eq!(report["bumped"], json!([1, 2]), "{report}");
    assert_eq!(
        report["running"],
        json!(["hook: start welcome"]),
        "{report}"
    );
    assert_eq!(
        report["lines"],
        json!(["hook: start welcome", "hook: stop welcome at 2"]),
        "{report}"
    );
}

#[tokio::test]
async fn a_kotlin_service_keeps_local_and_session_fields_across_a_reload() {
    let stored = KotlinMarkerProofs::stored_fields();
    assert_eq!(stored, json!({"draft": "session", "visits": "local"}));
    if !kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, TAC_KOTLIN_WASM_STDLIB or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    KotlinMarkerProofs::publish(
        root.path(),
        &[KotlinMarkerProofs::STATE_SOURCE],
        "draft",
        "DraftService",
    )
    .await;
    KotlinMarkerProofs::write(root.path(), "state.json", &stored.to_string());
    let report = KotlinMarkerProofs::drive(root.path(), KotlinMarkerProofs::STATE_DRIVER);
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
        json!({"tac:service:draft:/:visits": "3"}),
        "{report}"
    );
    assert_eq!(
        report["session"],
        json!({"tac:service:draft:/:draft": "\"chapter one\""}),
        "{report}"
    );
}
