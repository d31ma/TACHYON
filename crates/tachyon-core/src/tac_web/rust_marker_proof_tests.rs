#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) struct RustPage;

impl RustPage {
    pub(super) fn ready() -> bool {
        let target = Command::new("rustc")
            .args([
                "--print",
                "target-libdir",
                "--target",
                crate::native::RustCompanion::WEB_TARGET,
            ])
            .output()
            .is_ok_and(|output| {
                output.status.success()
                    && Path::new(String::from_utf8_lossy(&output.stdout).trim()).is_dir()
            });
        let ready = [target, NodeDriver::available()] == [true, true];
        assert!(
            ready || std::env::var_os("CI").is_none(),
            "CI must provide the wasm32 Rust target and node"
        );
        ready
    }

    fn write(root: &Path, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(&path, bytes).expect("file");
        path
    }

    pub(super) async fn publish(root: &Path, files: &[(&str, &str)], service: (&str, &str)) {
        for (relative, text) in files {
            Self::write(root, relative, text.as_bytes());
        }
        let (instance, class) = service;
        let services = [(
            String::from(instance),
            String::from(class),
            WebLanguage::Rust,
            root.join(format!("client/services/{class}.rs")),
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
        Self::write(
            root,
            "tac-storage.js",
            include_str!("../template/tac-storage.js").as_bytes(),
        );
    }

    pub(super) fn drive(root: &Path, driver: &str) -> serde_json::Value {
        Self::write(root, "drive.mjs", driver.as_bytes());
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
}

pub(super) struct RustMarkerProofs;

impl RustMarkerProofs {
    const LIFECYCLE_FILES: [(&'static str, &'static str); 2] = [
        (
            "client/models/OrderPlaced.rs",
            "#[derive(Clone, Debug, Default)]\npub struct OrderPlaced {\n    pub total: i64,\n}\n",
        ),
        (
            "client/services/JournalService.rs",
            "use crate::models::OrderPlaced;\n\n#[Service]\n#[derive(Default)]\npub struct JournalService {\n    application_event_publisher: ApplicationEventPublisher,\n    clipboard: Clipboard,\n    pub heard: Vec<String>,\n}\n\n\
             impl JournalService {\n    pub fn place(&mut self, total: i64) {\n        self.heard.push(format!(\"place {total}\"));\n        \
             self.application_event_publisher.publish_event(OrderPlaced { total }, false);\n        self.heard.push(format!(\"placed {total}\"));\n    }\n\n    \
             #[OnEvent]\n    #[InBackground]\n    fn audit(&mut self, order: OrderPlaced) {\n        self.heard.push(format!(\"audit {}\", order.total));\n    }\n\n    \
             #[OnEvent]\n    #[Order(2)]\n    fn record(&mut self, order: OrderPlaced) {\n        self.heard.push(format!(\"record {}\", order.total));\n    }\n\n    \
             #[OnEvent]\n    #[Order(1)]\n    fn check(&mut self, order: OrderPlaced) {\n        self.heard.push(format!(\"check {}\", order.total));\n    }\n\n    \
             #[OnStop]\n    fn stop(&mut self) {\n        let _ = self.clipboard.write_text(&format!(\"stopped after {}\", self.heard.len()));\n    }\n}\n",
        ),
    ];

    const LIFECYCLE_DRIVER: &'static str = "const asked = []\n\
         globalThis[Symbol.for('tachyon.tac.web')] = {\n\
         \x20 host: (text) => { asked.push(JSON.parse(text)); return JSON.stringify({ value: null }) },\n\
         \x20 calling: (owner, work) => work(),\n\
         }\n\
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
         const running = [...asked]\n\
         bus.dispose(journal)\n\
         await new Promise((settle) => setTimeout(settle, 50))\n\
         console.log(JSON.stringify({ returned, after, running, stopped: asked, failures }))\n\
         process.exit(0)\n";

    const STATE_SOURCE: (&'static str, &'static str) = (
        "client/services/DraftService.rs",
        "#[Service]\n#[derive(Default)]\npub struct DraftService {\n    #[LocalState]\n    visits: i64,\n    #[SessionState]\n    draft: String,\n}\n\n\
         impl DraftService {\n    pub fn visit(&mut self) -> i64 {\n        self.visits += 1;\n        self.visits\n    }\n\n    \
         pub fn write(&mut self, text: String) {\n        self.draft = text;\n    }\n\n    \
         pub fn grow(&mut self, times: i64) {\n        self.draft = \"x\".repeat(times as usize);\n    }\n}\n",
    );

    const STATE_DRIVER: &'static str = "import { readFile } from 'node:fs/promises'\n\
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
         \x20 return bindPersistentFields(draft, 'service:draft:/')\n\
         }\n\
         const tab = new MemoryStorage()\n\
         const page = open(tab)\n\
         await page.visit(); await page.visit()\n\
         await page.write('chapter one')\n\
         const refused = { assigned: '', grown: '' }\n\
         try { page.draft = 'x'.repeat(70000) } catch (error) { refused.assigned = error.name }\n\
         try { await page.grow(70000) } catch (error) { refused.grown = error.name }\n\
         const written = { saved: Object.fromEntries(local.items), session: Object.fromEntries(tab.items) }\n\
         const reloaded = open(tab)\n\
         const afterReload = { visits: reloaded.visits, draft: reloaded.draft, visit: await reloaded.visit() }\n\
         const other = open(new MemoryStorage())\n\
         const newTab = { visits: other.visits, draft: other.draft }\n\
         console.log(JSON.stringify({ refused, written, afterReload, newTab, saved: Object.fromEntries(local.items) }))\n\
         process.exit(0)\n";

    const CACHE_FILES: [(&'static str, &'static str); 4] = [
        (
            "client/models/Greeting.rs",
            "#[derive(Clone, Debug, Default)]\npub struct Greeting {\n    pub message: String,\n}\n",
        ),
        (
            "client/remotes/GreetingClient.rs",
            "use crate::models::Greeting;\n\n#[HTTPClient(\"/api\")]\n#[CacheSettings({\"name\":\"greetings\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":8})]\ntrait GreetingClient {\n    \
             #[Cached]\n    #[GetRequest(\"/greeting\")]\n    async fn read(&self) -> BehaviorResult<Greeting>;\n\n    \
             #[ClearCache({\"method\":\"read\"})]\n    #[PostRequest(\"/clear\")]\n    async fn clear(&self) -> BehaviorResult<Greeting>;\n}\n",
        ),
        (
            "client/setups/GreetingSetup.rs",
            "#[Setup]\nstruct GreetingSetup {}\nimpl GreetingSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        Ok(BehaviorConfiguration::default())\n    }\n}\n",
        ),
        (
            "client/services/GreetingService.rs",
            "use crate::models::Greeting;\n\n#[Service]\npub struct GreetingService {\n    #[Needs]\n    greeting_client: std::sync::Arc<GreetingClient>,\n    pub message: String,\n}\n\n\
             impl GreetingService {\n    pub async fn ask(&mut self) -> BehaviorResult<()> {\n        self.message = self.greeting_client.read().await?.message;\n        Ok(())\n    }\n\n    \
             pub async fn forget(&mut self) -> BehaviorResult<()> {\n        self.greeting_client.clear().await?;\n        Ok(())\n    }\n}\n",
        ),
    ];

    const CACHE_DRIVER: &'static str = "import { readFile } from 'node:fs/promises'\n\
         console.info = () => {}\n\
         globalThis.indexedDB = { databases: async () => [], open: () => ({}) }\n\
         ;(0, eval)(await readFile('./tac-storage.js', 'utf8'))\n\
         const started = []\n\
         globalThis[Symbol.for('tachyon.tac.http')] = { start: (text) => {\n\
         \x20 const request = JSON.parse(text)\n\
         \x20 let settle\n\
         \x20 const answer = new Promise((resolve) => { settle = resolve })\n\
         \x20 started.push({ request, settle })\n\
         \x20 return { answer, cancel: () => {} }\n\
         } }\n\
         const waitFor = async (count) => { for (let tries = 0; started.length < count && tries < 500; tries += 1) await new Promise((resolve) => setTimeout(resolve, 2)) }\n\
         const reply = async (count, value) => { await waitFor(count); started[count - 1].settle(JSON.stringify(value)) }\n\
         const { default: GreetingService } = await import('./GreetingService.js')\n\
         const greeting = new GreetingService({})\n\
         const asking = greeting.ask()\n\
         await reply(1, { value: { message: 'Hello' } })\n\
         await asking\n\
         const first = greeting.message\n\
         await greeting.ask()\n\
         const kept = { message: greeting.message, sent: started.length }\n\
         const forgetting = greeting.forget()\n\
         await reply(2, { value: { message: 'Cleared' } })\n\
         await forgetting\n\
         const askingAgain = greeting.ask()\n\
         await reply(3, { value: { message: 'Fresh' } })\n\
         await askingAgain\n\
         const requests = started.map(({ request }) => [request.method, request.path, request.revalidate === true])\n\
         console.log(JSON.stringify({ first, kept, requests, last: greeting.message }))\n\
         process.exit(0)\n";

    fn project() -> tempfile::TempDir {
        tempfile::tempdir().expect("project")
    }

    pub(super) async fn stored_state() -> (serde_json::Value, Option<serde_json::Value>) {
        let stored = Self::stored_fields();
        if !RustPage::ready() {
            eprintln!("skipping: the wasm32 Rust target or node is unavailable");
            return (stored, None);
        }
        let root = Self::project();
        let service = ("draft", "DraftService");
        RustPage::publish(root.path(), &[Self::STATE_SOURCE], service).await;
        std::fs::write(root.path().join("state.json"), stored.to_string()).expect("stored fields");
        let report = RustPage::drive(root.path(), Self::STATE_DRIVER);
        (stored, Some(report))
    }

    fn stored_fields() -> serde_json::Value {
        let (relative, source) = Self::STATE_SOURCE;
        let fields = crate::behavior::StoredState::fields(Path::new(relative), source)
            .unwrap_or_else(|failure| panic!("{failure}"));
        json!(fields)
    }
}

#[tokio::test]
async fn a_rust_web_service_runs_its_stop_hook_ordered_and_background_listeners() {
    if !RustPage::ready() {
        eprintln!("skipping: the wasm32 Rust target or node is unavailable");
        return;
    }
    let root = RustMarkerProofs::project();
    let service = ("journal", "JournalService");
    RustPage::publish(root.path(), &RustMarkerProofs::LIFECYCLE_FILES, service).await;
    let report = RustPage::drive(root.path(), RustMarkerProofs::LIFECYCLE_DRIVER);
    assert_eq!(
        report["returned"],
        json!(["place 3", "placed 3", "check 3", "record 3"]),
        "{report}"
    );
    assert_eq!(
        report["after"],
        json!(["place 3", "placed 3", "check 3", "record 3", "audit 3"]),
        "{report}"
    );
    assert_eq!(report["running"], json!([]), "{report}");
    assert_eq!(
        report["stopped"],
        json!([{"api": "clipboard", "call": "writeText", "args": ["stopped after 5"]}]),
        "{report}"
    );
    assert_eq!(report["failures"], 0, "{report}");
}

#[tokio::test]
async fn a_rust_cached_client_read_is_kept_until_clear_cache() {
    if !RustPage::ready() {
        eprintln!("skipping: the wasm32 Rust target or node is unavailable");
        return;
    }
    let root = RustMarkerProofs::project();
    let service = ("greeting", "GreetingService");
    RustPage::publish(root.path(), &RustMarkerProofs::CACHE_FILES, service).await;
    let report = RustPage::drive(root.path(), RustMarkerProofs::CACHE_DRIVER);
    assert_eq!(report["first"], "Hello", "{report}");
    assert_eq!(
        report["kept"],
        json!({"message": "Hello", "sent": 1}),
        "{report}"
    );
    assert_eq!(
        report["requests"],
        json!([
            ["GET", "/api/greeting", false],
            ["POST", "/api/clear", false],
            ["GET", "/api/greeting", true],
        ]),
        "{report}"
    );
    assert_eq!(report["last"], "Fresh", "{report}");
}
