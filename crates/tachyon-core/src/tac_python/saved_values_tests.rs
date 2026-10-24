use super::page_tests::PythonPage;
use std::path::Path;

struct PythonSavedValues;

impl PythonSavedValues {
    const VALUES: [(&'static str, &'static str); 1] = [(
        "client/models/PublicValues.py",
        "from typing import Annotated\n\n\nclass PublicValues:\n    title: Annotated[str, setting(\"${tac.catalog.title:Catalog}\")] = ''\n",
    )];

    const SETTINGS_SERVICE: &'static str = "@service\nclass SettingsService:\n    \
         def values(self, title: str) -> dict[str, object]:\n        \
         application = {'tac': {'catalog': {'title': title}}} if title else {'tac': {'other': {'size': 1}}}\n        \
         return Behaviors.values(PublicValues, application)\n";

    const SETTINGS_DRIVER: &'static str = "console.info = () => {}\n\
         const { default: SettingsService } = await import('./SettingsService.js')\n\
         const settings = new SettingsService({})\n\
         console.log(JSON.stringify({ values: [settings.values('Shop'), settings.values('')] }))\n\
         process.exit(0)\n";

    const SHELF_SERVICE: &'static str = "from typing import Annotated\n\n@service\nclass ShelfService:\n    \
         visits: Annotated[int, local_state] = 0\n    draft: Annotated[str, session_state] = ''\n\n    \
         def visit(self) -> int:\n        self.visits += 1\n        return self.visits\n\n    \
         def keep(self, text: str) -> str:\n        self.draft = text\n        return self.draft\n";

    const SHELF_DRIVER: &'static str = "console.info = () => {}\n\
         const client = await readFile('./tac-client.js', 'utf8')\n\
         const host = client.slice(client.indexOf('const webStorage = '), client.indexOf('const persistScope = '))\n\
         class MemoryStorage {\n\
         \x20 constructor() { this.items = new Map() }\n\
         \x20 getItem(key) { return this.items.has(key) ? this.items.get(key) : null }\n\
         \x20 setItem(key, value) { this.items.set(key, String(value)) }\n\
         \x20 removeItem(key) { this.items.delete(key) }\n\
         }\n\
         const local = new MemoryStorage()\n\
         const { default: ShelfService } = await import('./ShelfService.js')\n\
         const stored = JSON.parse(await readFile('./state.json', 'utf8'))\n\
         const open = (session) => {\n\
         \x20 const { Owners, bindPersistentFields } = new Function('localStorage', 'sessionStorage', `${host}\\nreturn { Owners, bindPersistentFields }`)(local, session)\n\
         \x20 const shelf = new ShelfService({})\n\
         \x20 Owners.stored.set(shelf, stored)\n\
         \x20 bindPersistentFields(shelf, 'service:shelf:/')\n\
         \x20 return shelf\n\
         }\n\
         const tab = new MemoryStorage()\n\
         const page = open(tab)\n\
         page.visit(); page.visit()\n\
         page.keep('chapter one')\n\
         page.__tachyonDispose()\n\
         const reloaded = open(tab)\n\
         const afterReload = { visits: reloaded.visits, draft: reloaded.draft, visit: reloaded.visit() }\n\
         reloaded.__tachyonDispose()\n\
         const other = open(new MemoryStorage())\n\
         const newTab = { visits: other.visits, draft: other.draft }\n\
         other.__tachyonDispose()\n\
         console.log(JSON.stringify({ afterReload, newTab, saved: Object.fromEntries(local.items), session: Object.fromEntries(tab.items) }))\n\
         process.exit(0)\n";

    fn stored_fields() -> serde_json::Value {
        let fields = crate::behavior::StoredState::fields(
            Path::new("client/services/ShelfService.py"),
            Self::SHELF_SERVICE,
        )
        .unwrap_or_else(|error| panic!("stored fields: {error:?}"));
        serde_json::json!(fields)
    }
}

#[tokio::test]
async fn a_python_setting_field_is_bound_from_the_application_values() {
    if let Some(reason) = PythonPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    for (relative, text) in PythonSavedValues::VALUES {
        PythonPage::write(root.path(), relative, text);
    }
    PythonPage::publish(
        root.path(),
        &[(
            "settings",
            "SettingsService",
            PythonSavedValues::SETTINGS_SERVICE,
        )],
    )
    .await;
    let report = PythonPage::run(root.path(), PythonSavedValues::SETTINGS_DRIVER);
    assert_eq!(
        report["values"],
        serde_json::json!([{"title": "Shop"}, {"title": "Catalog"}]),
        "{report}"
    );
}

#[tokio::test]
async fn a_python_session_state_field_survives_a_reload_but_not_a_new_tab() {
    let stored = PythonSavedValues::stored_fields();
    assert_eq!(
        stored,
        serde_json::json!({"draft": "session", "visits": "local"})
    );
    if let Some(reason) = PythonPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    PythonPage::publish(
        root.path(),
        &[("shelf", "ShelfService", PythonSavedValues::SHELF_SERVICE)],
    )
    .await;
    PythonPage::write(
        root.path(),
        "tac-client.js",
        include_str!("../template/tac-client.js"),
    );
    PythonPage::write(root.path(), "state.json", &stored.to_string());
    let report = PythonPage::run(root.path(), PythonSavedValues::SHELF_DRIVER);
    assert_eq!(
        report["afterReload"],
        serde_json::json!({"visits": 2, "draft": "chapter one", "visit": 3}),
        "{report}"
    );
    assert_eq!(
        report["newTab"],
        serde_json::json!({"visits": 3, "draft": ""}),
        "{report}"
    );
    assert_eq!(
        report["saved"],
        serde_json::json!({"tac:service:shelf:/:visits": "3"}),
        "{report}"
    );
    assert_eq!(
        report["session"],
        serde_json::json!({"tac:service:shelf:/:draft": "\"chapter one\""}),
        "{report}"
    );
}
