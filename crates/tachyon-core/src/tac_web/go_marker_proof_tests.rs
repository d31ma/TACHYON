#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) struct GoMarkerProofs;

impl GoMarkerProofs {
    const STATE_FILES: [(&str, &str); 1] = [(
        "client/services/DraftService.go",
        "// @Service\ntype DraftService struct {\n\t// @LocalState\n\tVisits int\n\t// @SessionState\n\tDraft string\n}\n\n\
         func (draft *DraftService) Visit() int {\n\tdraft.Visits++\n\treturn draft.Visits\n}\n\n\
         func (draft *DraftService) Write(text string) {\n\tdraft.Draft = text\n}\n",
    )];

    const STATE_DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
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
         \x20 return bindPersistentFields(draft, 'service:draft:/')\n\
         }\n\
         const tab = new MemoryStorage()\n\
         const page = open(tab)\n\
         await page.visit(); await page.visit()\n\
         await page.write('chapter one')\n\
         const written = { saved: Object.fromEntries(local.items), session: Object.fromEntries(tab.items) }\n\
         const reloaded = open(tab)\n\
         const afterReload = { visits: reloaded.visits, draft: reloaded.draft, visit: await reloaded.visit() }\n\
         const other = open(new MemoryStorage())\n\
         const newTab = { visits: other.visits, draft: other.draft }\n\
         console.log(JSON.stringify({ written, afterReload, newTab, saved: Object.fromEntries(local.items) }))\n\
         process.exit(0)\n";

    const NEEDED_FILES: [(&str, &str); 2] = [
        (
            "client/services/NotesService.go",
            "// @Service\ntype NotesService struct {\n\t// @LocalState\n\tVisits int\n\t// @SessionState\n\tDraft string\n}\n\n\
             func (notes *NotesService) Keep(text string) {\n\tnotes.Visits++\n\tnotes.Draft = text\n}\n",
        ),
        (
            "client/services/EditorService.go",
            "// @Service\ntype EditorService struct {\n\t// @Needs\n\tnotes *NotesService\n}\n\n\
             func (editor *EditorService) Write(text string) {\n\teditor.notes.Keep(text)\n}\n\n\
             func (editor *EditorService) Draft() string {\n\treturn editor.notes.Draft\n}\n\n\
             func (editor *EditorService) Visits() int {\n\treturn editor.notes.Visits\n}\n",
        ),
    ];

    const BACKGROUND_FILES: [(&str, &str); 2] = [
        (
            "client/models/OrderPlaced.go",
            "type OrderPlaced struct {\n\tTotal int `json:\"total\"`\n}\n",
        ),
        (
            "client/services/JournalService.go",
            "import \"fmt\"\n\n// @Service\ntype JournalService struct {\n\tHeard     []string\n\tpublisher ApplicationEventPublisher\n}\n\n\
             func (journal *JournalService) Place(total int) {\n\tjournal.Heard = append(journal.Heard, fmt.Sprint(\"place \", total))\n\tjournal.publisher.PublishEvent(OrderPlaced{Total: total}, false)\n\tjournal.Heard = append(journal.Heard, fmt.Sprint(\"placed \", total))\n}\n\n\
             // @OnEvent\n// @InBackground\nfunc (journal *JournalService) Audit(order OrderPlaced) {\n\tjournal.Heard = append(journal.Heard, fmt.Sprint(\"audit \", order.Total))\n}\n\n\
             // @OnEvent\nfunc (journal *JournalService) Record(order OrderPlaced) {\n\tjournal.Heard = append(journal.Heard, fmt.Sprint(\"record \", order.Total))\n}\n",
        ),
    ];

    const BACKGROUND_DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
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
         await new Promise((settle) => setTimeout(settle, 50))\n\
         const after = [...journal.heard]\n\
         const listeners = plan.types[0].listeners.map(({ method, asynchronous }) => [method.split('_').pop(), asynchronous === true])\n\
         console.log(JSON.stringify({ returned, after, listeners, failures }))\n\
         process.exit(0)\n";

    fn tools_available() -> bool {
        let go = Command::new("go")
            .arg("version")
            .output()
            .is_ok_and(|output| output.status.success());
        go && NodeDriver::available()
    }

    fn write(root: &Path, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(&path, bytes).expect("file");
        path
    }

    const START_FILES: [(&str, &str); 4] = [
        (
            "client/models/Note.go",
            "type Note struct {\n\tText string `json:\"text\"`\n}\n",
        ),
        (
            "client/setups/NoteSetup.go",
            "// @Setup\ntype NoteSetup struct{}\n\nfunc (NoteSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {\n\treturn NewBehaviorConfiguration(), nil\n}\n",
        ),
        (
            "client/remotes/NoteClient.go",
            "// @HTTPClient(\"/api\")\ntype NoteClient interface {\n\t// @GetRequest(\"/note\")\n\tRead() (Note, error)\n}\n",
        ),
        (
            "client/services/StartService.go",
            "// @Service\ntype StartService struct {\n\t// @Needs\n\tnoteClient *NoteClient\n\tRequest    string\n}\n\n\
             // @OnStart\nfunc (start *StartService) Start() {\n\t_, failure := start.noteClient.Read()\n\tstart.Request = failure.Error()\n}\n",
        ),
    ];

    const START_DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const { default: StartService, __tachyon_event_plan__: plan } = await import('./StartService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => {} })\n\
         const start = new StartService({})\n\
         bus.attach(start, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(start)\n\
         console.log(JSON.stringify({ request: start.request }))\n\
         process.exit(0)\n";

    async fn publish(root: &Path, files: &[(&str, &str)], class: &str) {
        let mut source = PathBuf::new();
        for (relative, text) in files {
            source = Self::write(root, relative, text.as_bytes());
        }
        let instance = class.trim_end_matches("Service").to_ascii_lowercase();
        let services = [(instance, String::from(class), WebLanguage::Go, source)];
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

    fn drive(root: &Path, driver: &str) -> serde_json::Value {
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

    pub(super) async fn stored_state() -> (serde_json::Value, Option<serde_json::Value>) {
        let stored = Self::stored_fields();
        if !Self::tools_available() {
            assert!(
                std::env::var_os("CI").is_none(),
                "CI must provide go and node"
            );
            eprintln!("skipping: go or node is unavailable");
            return (stored, None);
        }
        let root = tempfile::tempdir().expect("project");
        Self::publish(root.path(), &Self::STATE_FILES, "DraftService").await;
        Self::write(root.path(), "state.json", stored.to_string().as_bytes());
        let report = Self::drive(root.path(), Self::STATE_DRIVER);
        (stored, Some(report))
    }

    fn stored_fields() -> serde_json::Value {
        let (relative, source) = Self::STATE_FILES[0];
        let fields = crate::behavior::StoredState::fields(Path::new(relative), source)
            .unwrap_or_else(|failure| panic!("{failure}"));
        json!(fields)
    }
}

#[tokio::test]
async fn a_go_in_background_listener_runs_after_the_publisher_returns() {
    if !GoMarkerProofs::tools_available() {
        assert!(
            std::env::var_os("CI").is_none(),
            "CI must provide go and node"
        );
        eprintln!("skipping: go or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    GoMarkerProofs::publish(
        root.path(),
        &GoMarkerProofs::BACKGROUND_FILES,
        "JournalService",
    )
    .await;
    let report = GoMarkerProofs::drive(root.path(), GoMarkerProofs::BACKGROUND_DRIVER);
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
    assert_eq!(
        report["listeners"],
        json!([["Audit", true], ["Record", false]]),
        "{report}"
    );
    assert_eq!(report["failures"], 0, "{report}");
}

#[tokio::test]
async fn a_go_request_in_on_start_explains_why_it_cannot_wait() {
    if !GoMarkerProofs::tools_available() {
        assert!(
            std::env::var_os("CI").is_none(),
            "CI must provide go and node"
        );
        eprintln!("skipping: go or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    GoMarkerProofs::publish(root.path(), &GoMarkerProofs::START_FILES, "StartService").await;
    let report = GoMarkerProofs::drive(root.path(), GoMarkerProofs::START_DRIVER);
    assert_eq!(
        report["request"],
        json!(
            "HTTP_REQUEST: A browser HTTP request or socket send cannot wait in OnStart, a getter or a setter. Publish an event from OnStart and make the request in an @OnEvent method."
        ),
        "{report}"
    );
}

#[tokio::test]
async fn a_go_service_that_another_go_service_needs_keeps_its_saved_fields() {
    if !GoMarkerProofs::tools_available() {
        assert!(
            std::env::var_os("CI").is_none(),
            "CI must provide go and node"
        );
        eprintln!("skipping: go or node is unavailable");
        return;
    }
    let proof = super::needed_state_proof::NeededStateProof {
        language: WebLanguage::Go,
        files: &GoMarkerProofs::NEEDED_FILES,
        prefix: super::needed_state_proof::NeededStateProof::FETCH,
        setup: |_| {},
    };
    let report = proof.run().await;
    super::needed_state_proof::NeededStateProof::assert_saved(&report);
}
