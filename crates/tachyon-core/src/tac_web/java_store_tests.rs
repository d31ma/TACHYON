#![allow(clippy::expect_used)]

use super::java_tests::{java_web_ready, publish, run_driver, write};
use serde_json::json;
use std::path::Path;

struct ShelfProject;

impl ShelfProject {
    const SERVICE: (&str, &str) = (
        "client/services/ShelfService.java",
        r#"import java.util.Map;

@Service
@CacheSettings({"name":"shelf","ttlMs":60000,"maxBytes":4096,"maxEntries":8})
class ShelfService {
    @LocalState
    int visits = 0;
    @SessionState
    String draft = "";
    Behaviors.Context behaviors;
    @Needs
    BookStore bookStore;
    @Needs
    BookConverter bookConverter;
    private long reads = 0;

    @OnStart
    void open() {
        behaviors = Behaviors.create("ShelfSetup", new Behaviors.Options("shelf", new Behaviors.Signal()));
    }

    int visit() {
        visits += 1;
        return visits;
    }

    @Cached
    Object price(@CacheKey String sku, String note) {
        reads += 1;
        return Map.of("sku", sku, "note", note, "reads", reads);
    }

    @UpdateCache({"method":"price"})
    Object reprice(String sku, long reads) {
        return Map.of("sku", sku, "note", "written", "reads", reads);
    }

    Object add(String id, String status) {
        return behaviors.repository("BookEntity").save(Map.of("id", id, "status", status));
    }

    Object oversized() {
        return bookStore.excluding("active", Map.of("size", 4));
    }

    Object report() {
        return Map.of(
            "active", bookStore.findByStatus("active"),
            "count", bookStore.countByStatus("active"),
            "others", bookStore.excluding("active", Map.of()),
            "full", bookStore.excluding("active", Map.of("size", 3)),
            "card", bookConverter.toCard(Map.of("id", "b1", "detail", Map.of("title", "Dune"), "status", "active")));
    }
}
"#,
    );

    const FILES: [(&str, &str); 4] = [
        (
            "client/setups/ShelfSetup.java",
            "@Setup\nclass ShelfSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return new Behaviors.Configuration();\n    }\n}\n",
        ),
        (
            "client/entities/BookEntity.java",
            "@Entity({\"adapter\":\"indexeddb\"})\n@Table(\"books\")\nclass BookEntity {\n    @Id\n    String id = \"\";\n    String status = \"\";\n}\n",
        ),
        (
            "client/stores/BookStore.java",
            r#"@Store(BookEntity.class, {"fields":["id","status"],"defaultSize":2,"maxSize":3,"setup":"ShelfSetup"})
interface BookStore {
    Object findByStatus(String status);

    Object countByStatus(String status);

    @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
    Object excluding(String status, @PageDefaults java.util.Map<String, Object> page);
}
"#,
        ),
        (
            "client/converters/BookConverter.java",
            r#"@Converter
interface BookConverter {
    @Convert({"fields":[{"source":"detail.title","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"book"}]})
    Object toCard(Object book);
}
"#,
        ),
    ];

    const STORAGE_DRIVER: &str = r"import { readFile } from 'node:fs/promises'
globalThis.fetch = async (url) => new Response(await readFile(url))
console.info = () => {}
globalThis.indexedDB = { databases: async () => [], open: () => ({}) }
;(0, eval)(await readFile('./tac-storage.js', 'utf8'))
const { default: ShelfService } = await import('./ShelfService.js')
const shelf = new ShelfService({})
shelf.__tachyonStarted()
const first = shelf.price('anvil', 'one')
const sameKey = shelf.price('anvil', 'two')
const otherKey = shelf.price('hammer', 'one')
const written = shelf.reprice('anvil', 99)
const updated = shelf.price('anvil', 'three')
for (const [id, status] of [['b1', 'active'], ['b2', 'retired'], ['b3', 'active'], ['b4', 'draft'], ['b5', 'lost'], ['b6', 'lent']]) shelf.add(id, status)
const report = shelf.report()
let oversized = ''
try { shelf.oversized() } catch (error) { oversized = error.message }
shelf.__tachyonDispose()
console.log(JSON.stringify({ first, sameKey, otherKey, written, updated, oversized, ...report }))
process.exit(0)
";

    const STATE_DRIVER: &str = r"import { readFile } from 'node:fs/promises'
globalThis.fetch = async (url) => new Response(await readFile(url))
console.info = () => {}
globalThis.indexedDB = { databases: async () => [], open: () => ({}) }
;(0, eval)(await readFile('./tac-storage.js', 'utf8'))
const client = await readFile('./tac-client.js', 'utf8')
const host = client.slice(client.indexOf('const webStorage = '), client.indexOf('const persistScope = '))
class MemoryStorage {
  constructor() { this.items = new Map() }
  getItem(key) { return this.items.has(key) ? this.items.get(key) : null }
  setItem(key, value) { this.items.set(key, String(value)) }
  removeItem(key) { this.items.delete(key) }
}
const local = new MemoryStorage()
const open = (session, stored) => {
  const { Owners, bindPersistentFields } = new Function('localStorage', 'sessionStorage', `${host}\nreturn { Owners, bindPersistentFields }`)(local, session)
  const shelf = new ShelfService({})
  Owners.stored.set(shelf, stored)
  bindPersistentFields(shelf, 'service:shelf:/')
  shelf.__tachyonStarted()
  return shelf
}
const { default: ShelfService } = await import('./ShelfService.js')
const stored = JSON.parse(await readFile('./state.json', 'utf8'))
const tab = new MemoryStorage()
const page = open(tab, stored)
page.visit(); page.visit()
page.draft = 'chapter one'
page.__tachyonDispose()
const reloaded = open(tab, stored)
const afterReload = { visits: reloaded.visits, draft: reloaded.draft, visit: reloaded.visit() }
reloaded.__tachyonDispose()
const other = open(new MemoryStorage(), stored)
const newTab = { visits: other.visits, draft: other.draft }
other.__tachyonDispose()
console.log(JSON.stringify({ afterReload, newTab, saved: Object.fromEntries(local.items), session: Object.fromEntries(tab.items) }))
process.exit(0)
";

    async fn built(root: &Path) {
        for (relative, text) in Self::FILES.iter().chain([&Self::SERVICE]) {
            write(root, relative, text);
        }
        write(
            root,
            "tac-storage.js",
            include_str!("../template/tac-storage.js"),
        );
        write(
            root,
            "tac-client.js",
            include_str!("../template/tac-client.js"),
        );
        publish(root, "shelf", "ShelfService").await;
    }

    fn stored_fields() -> serde_json::Value {
        let (relative, source) = Self::SERVICE;
        let fields = crate::behavior::StoredState::fields(Path::new(relative), source)
            .unwrap_or_else(|failure| panic!("{failure}"));
        json!(fields)
    }
}

#[tokio::test]
async fn a_java_web_service_caches_by_key_queries_its_store_and_maps_with_a_converter() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    ShelfProject::built(root.path()).await;
    let report = run_driver(root.path(), ShelfProject::STORAGE_DRIVER);
    let price = |note: &str, reads: i64| json!({"sku": "anvil", "note": note, "reads": reads});
    assert_eq!(report["first"], price("one", 1), "{report}");
    assert_eq!(report["sameKey"], price("one", 1), "{report}");
    assert_eq!(
        report["otherKey"],
        json!({"sku": "hammer", "note": "one", "reads": 2}),
        "{report}"
    );
    assert_eq!(report["written"], price("written", 99), "{report}");
    assert_eq!(report["updated"], price("written", 99), "{report}");
    let book = |id: &str, status: &str| json!({"id": id, "status": status});
    assert_eq!(
        report["active"],
        json!([book("b1", "active"), book("b3", "active")]),
        "{report}"
    );
    assert_eq!(report["count"], 2, "{report}");
    assert_eq!(
        report["others"],
        json!([book("b6", "lent"), book("b5", "lost")]),
        "{report}"
    );
    assert_eq!(
        report["full"],
        json!([book("b6", "lent"), book("b5", "lost"), book("b4", "draft")]),
        "{report}"
    );
    assert_eq!(
        report["oversized"], "Web service shelf: pagination",
        "{report}"
    );
    assert_eq!(
        report["card"],
        json!({"title": "Dune", "absent": null, "kind": "book"}),
        "{report}"
    );
}

#[tokio::test]
async fn a_java_web_service_keeps_local_and_session_fields_across_a_reload() {
    let stored = ShelfProject::stored_fields();
    assert_eq!(stored, json!({"draft": "session", "visits": "local"}));
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    ShelfProject::built(root.path()).await;
    write(root.path(), "state.json", &stored.to_string());
    let report = run_driver(root.path(), ShelfProject::STATE_DRIVER);
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
