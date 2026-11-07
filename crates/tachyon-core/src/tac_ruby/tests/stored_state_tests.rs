use super::{Page, write};
use serde_json::json;
use std::path::Path;

struct StoredStatePage;

impl StoredStatePage {
    const SHELF: &'static str = "#[Service]\nclass ShelfService\n  #[LocalState]\n  # @return [Integer]\n  attr_accessor :visits\n\n  #[SessionState]\n  # @return [String]\n  attr_accessor :draft_note\n\n  def initialize\n    @visits = 0\n    @draft_note = \"\"\n  end\n\n  # @return [Integer]\n  def visit\n    @visits += 1\n  end\nend\n";

    const DRIVER: &'static str = "const client = await readFile('./tac-client.js', 'utf8')\n\
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
         page.draftNote = 'chapter one'\n\
         page.__tachyonDispose()\n\
         const reloaded = open(tab)\n\
         const afterReload = { visits: reloaded.visits, draftNote: reloaded.draftNote, visit: reloaded.visit() }\n\
         reloaded.__tachyonDispose()\n\
         const other = open(new MemoryStorage())\n\
         const newTab = { visits: other.visits, draftNote: other.draftNote }\n\
         other.__tachyonDispose()\n\
         const unbound = new ShelfService({})\n\
         const fresh = unbound.visits\n\
         unbound.__tachyonDispose()\n\
         console.log(JSON.stringify({ fresh, visited, afterReload, newTab, saved: Object.fromEntries(local.items), session: Object.fromEntries(tab.items) }))\n\
         process.exit(0)\n";

    fn page_storage(root: &Path) -> serde_json::Value {
        write(
            root,
            "tac-client.js",
            include_str!("../../template/tac-client.js"),
        );
        let registry = crate::template::ServiceRegistry::discover(root)
            .unwrap_or_else(|failure| panic!("{failure}"));
        let stored = registry
            .get("shelf")
            .unwrap_or_else(|| panic!("shelf service"))
            .stored();
        let state = json!(stored);
        write(root, "state.json", &state.to_string());
        state
    }
}

#[tokio::test]
async fn a_ruby_service_keeps_local_and_session_fields_across_a_reload() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    Page::publish(
        root.path(),
        &[("shelf", "ShelfService", StoredStatePage::SHELF)],
    )
    .await;
    let stored = StoredStatePage::page_storage(root.path());
    assert_eq!(stored, json!({"draftNote": "session", "visits": "local"}));
    let report = Page::run(root.path(), StoredStatePage::DRIVER);
    assert_eq!(report["fresh"], 0, "{report}");
    assert_eq!(report["visited"], json!([1, 2]), "{report}");
    assert_eq!(
        report["afterReload"],
        json!({"visits": 2, "draftNote": "chapter one", "visit": 3}),
        "{report}"
    );
    assert_eq!(
        report["newTab"],
        json!({"visits": 3, "draftNote": ""}),
        "{report}"
    );
    assert_eq!(
        report["saved"],
        json!({"tac:service:shelf:/:visits": "3"}),
        "{report}"
    );
    assert_eq!(
        report["session"],
        json!({"tac:service:shelf:/:draftNote": "\"chapter one\""}),
        "{report}"
    );
}
