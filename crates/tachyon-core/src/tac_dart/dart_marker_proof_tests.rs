use super::dart_stream_tests::DartStreams;
use serde_json::json;
use std::path::Path;

struct DartMarkerProofs;

impl DartMarkerProofs {
    const CONVERTER_FILES: [(&'static str, &'static str); 8] = [
        (
            "client/models/Detail.dart",
            include_str!(
                "../../../../website/content/docs/annotations/typed-mapping-tac/client/models/Detail.dart"
            ),
        ),
        (
            "client/models/ShortDetail.dart",
            include_str!(
                "../../../../website/content/docs/annotations/typed-mapping-tac/client/models/ShortDetail.dart"
            ),
        ),
        (
            "client/models/Product.dart",
            include_str!(
                "../../../../website/content/docs/annotations/typed-mapping-tac/client/models/Product.dart"
            ),
        ),
        (
            "client/models/Summary.dart",
            include_str!(
                "../../../../website/content/docs/annotations/typed-mapping-tac/client/models/Summary.dart"
            ),
        ),
        (
            "client/converters/ProductConverter.dart",
            include_str!(
                "../../../../website/content/docs/annotations/typed-mapping-tac/client/converters/ProductConverter.dart"
            ),
        ),
        (
            "client/services/CatalogService.dart",
            include_str!(
                "../../../../website/content/docs/annotations/typed-mapping-tac/client/services/CatalogService.dart"
            ),
        ),
        (
            "client/converters/CardConverter.dart",
            "@Converter\nabstract interface class CardConverter {\n  \
             @Convert({\"fields\":[{\"source\":\"detail.title\",\"target\":\"title\"},{\"source\":\"missing.deep\",\"target\":\"absent\"},{\"target\":\"kind\",\"constant\":\"book\"}]})\n  \
             Object? toCard(Object? book);\n}\n",
        ),
        (
            "client/services/CardService.dart",
            "@Service\nclass CardService {\n  @Needs\n  final CardConverter cardConverter;\n\n  \
             Object? card() {\n    return cardConverter.toCard({'id': 'b1', 'detail': {'title': 'Dune'}, 'status': 'active'});\n  }\n}\n",
        ),
    ];

    const CONVERTER_SERVICES: [(&'static str, &'static str); 2] = [
        ("CatalogService", "client/services/CatalogService.dart"),
        ("CardService", "client/services/CardService.dart"),
    ];

    const CONVERTER_DRIVER: &'static str = "const { default: CatalogService } = await import('./CatalogService.js')\n\
         const { default: CardService } = await import('./CardService.js')\n\
         const catalog = new CatalogService({})\n\
         const before = catalog.result\n\
         await catalog.load()\n\
         const card = await new CardService({}).card()\n\
         console.log(JSON.stringify({ before, result: catalog.result, card }))\n\
         process.exit(0)\n";

    const NEEDS_ALL_FILES: [(&'static str, &'static str); 1] = [(
        "client/services/PagerService.dart",
        "@Service\n@NeedsAll\nclass PagerService {\n  late final String path;\n  late final int size;\n  int turns = 0;\n\n  \
         String describe() {\n    turns += 1;\n    return '$path:$size:$turns';\n  }\n}\n",
    )];

    const NEEDS_ALL_SERVICES: [(&'static str, &'static str); 1] =
        [("PagerService", "client/services/PagerService.dart")];

    const NEEDS_ALL_DRIVER: &'static str = "const module = await import('./PagerService.js')\n\
         const signal = new AbortController().signal\n\
         const docs = module.__tachyon_create_owner__({ path: '/docs', size: 20 }, signal)\n\
         const shop = module.__tachyon_create_owner__({ path: '/shop', size: 5 }, signal)\n\
         console.log(JSON.stringify({ docs: [docs.path, docs.size, await docs.describe(), await docs.describe()], shop: [shop.path, shop.size, await shop.describe()] }))\n\
         process.exit(0)\n";

    const STATE_SOURCE: (&'static str, &'static str) = (
        "client/services/DraftService.dart",
        "@Service\nclass DraftService {\n  @LocalState\n  int visits = 0;\n  @SessionState\n  String draft = '';\n\n  \
         int visit() {\n    visits += 1;\n    return visits;\n  }\n\n  \
         void write(String text) {\n    draft = text;\n  }\n}\n",
    );

    const STATE_SERVICES: [(&'static str, &'static str); 1] =
        [("DraftService", "client/services/DraftService.dart")];

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
         const written = { saved: Object.fromEntries(local.items), session: Object.fromEntries(tab.items) }\n\
         const reloaded = open(tab)\n\
         const afterReload = { visits: reloaded.visits, draft: reloaded.draft, visit: await reloaded.visit() }\n\
         const other = open(new MemoryStorage())\n\
         const newTab = { visits: other.visits, draft: other.draft }\n\
         console.log(JSON.stringify({ written, afterReload, newTab, saved: Object.fromEntries(local.items), session: Object.fromEntries(tab.items) }))\n\
         process.exit(0)\n";

    const RETRY_FILES: [(&'static str, &'static str); 4] = [
        (
            "client/setups/OrdersSetup.dart",
            include_str!(
                "../../../../website/content/docs/annotations/retries-tac/client/setups/OrdersSetup.dart"
            ),
        ),
        (
            "client/services/OrdersService.dart",
            include_str!(
                "../../../../website/content/docs/annotations/retries-tac/client/services/OrdersService.dart"
            ),
        ),
        (
            "client/setups/FlakySetup.dart",
            "@Setup\nclass FlakySetup {\n  static BehaviorAdapters configure(BehaviorExecution execution) {\n    return BehaviorAdapters();\n  }\n}\n",
        ),
        (
            "client/services/FlakyService.dart",
            "@Service\nclass FlakyService {\n  late BehaviorContext behaviors;\n  int attempts = 0;\n  int plainAttempts = 0;\n\n  \
             @OnStart\n  void open() {\n    behaviors = Behaviors.create(FlakySetup, partition: 'flaky', signal: BehaviorSignal());\n  }\n\n  \
             @Retry({\"maxRetries\":2,\"delayMs\":1,\"multiplier\":1,\"maxDelayMs\":5,\"jitterMs\":0})\n  Future<Object?> never() async {\n    attempts += 1;\n    throw StateError('still down');\n  }\n\n  \
             Future<Object?> once() async {\n    plainAttempts += 1;\n    throw StateError('still down');\n  }\n}\n",
        ),
    ];

    const RETRY_SERVICES: [(&'static str, &'static str); 2] = [
        ("OrdersService", "client/services/OrdersService.dart"),
        ("FlakyService", "client/services/FlakyService.dart"),
    ];

    const RETRY_DRIVER: &'static str = "import { readFile } from 'node:fs/promises'\n\
         console.info = () => {}\n\
         globalThis.indexedDB = { databases: async () => [], open: () => ({}) }\n\
         ;(0, eval)(await readFile('./tac-storage.js', 'utf8'))\n\
         const { default: OrdersService, __tachyon_event_plan__: ordersPlan } = await import('./OrdersService.js')\n\
         const { default: FlakyService, __tachyon_event_plan__: flakyPlan } = await import('./FlakyService.js')\n\
         const outcome = (run) => Promise.resolve().then(run).then((value) => ({ value }), (error) => ({ error: String(error?.message ?? error) }))\n\
         const orders = new OrdersService({})\n\
         orders[ordersPlan.types[0].post_construct]()\n\
         const submitted = await outcome(() => orders.submit('o1'))\n\
         const flaky = new FlakyService({})\n\
         flaky[flakyPlan.types[0].post_construct]()\n\
         const exhausted = await outcome(() => flaky.never())\n\
         const plain = await outcome(() => flaky.once())\n\
         console.log(JSON.stringify({ submitted, orderAttempts: orders.attempts, exhausted, attempts: flaky.attempts, plain, plainAttempts: flaky.plainAttempts }))\n\
         process.exit(0)\n";

    const BACKGROUND_FILES: [(&'static str, &'static str); 2] = [
        (
            "client/models/OrderPlaced.dart",
            "class OrderPlaced {\n  final int total;\n\n  const OrderPlaced(this.total);\n}\n",
        ),
        (
            "client/services/JournalService.dart",
            "import 'models/OrderPlaced.dart';\n\n@Service\nclass JournalService {\n  @Needs\n  final ApplicationEventPublisher applicationEventPublisher;\n  List<String> heard = [];\n\n  \
             void place(int total) {\n    heard = [...heard, 'place $total'];\n    applicationEventPublisher.publishEvent(OrderPlaced(total));\n    heard = [...heard, 'placed $total'];\n  }\n\n  \
             @OnEvent\n  @InBackground\n  Future<void> audit(OrderPlaced order) async {\n    heard = [...heard, 'audit ${order.total}'];\n  }\n\n  \
             @OnEvent\n  void record(OrderPlaced order) {\n    heard = [...heard, 'record ${order.total}'];\n  }\n}\n",
        ),
    ];

    const BACKGROUND_SERVICES: [(&'static str, &'static str); 1] =
        [("JournalService", "client/services/JournalService.dart")];

    const BACKGROUND_DRIVER: &'static str = "const { default: JournalService, __tachyon_event_plan__: plan } = await import('./JournalService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.js')\n\
         let failures = 0\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { failures += 1 } })\n\
         const journal = new JournalService({})\n\
         bus.attach(journal, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(journal)\n\
         journal.place(3)\n\
         const returned = [...journal.heard]\n\
         await new Promise((settle) => setTimeout(settle, 20))\n\
         const after = [...journal.heard]\n\
         const listeners = plan.types[0].listeners.map(({ method, asynchronous }) => [method, asynchronous === true])\n\
         console.log(JSON.stringify({ returned, after, listeners, failures }))\n\
         process.exit(0)\n";

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).unwrap_or_else(|failure| panic!("folder: {failure}"));
        }
        std::fs::write(&path, text).unwrap_or_else(|failure| panic!("file: {failure}"));
    }

    fn project() -> tempfile::TempDir {
        tempfile::tempdir().unwrap_or_else(|failure| panic!("project: {failure}"))
    }

    fn with_page_runtime(root: &Path) {
        Self::write(
            root,
            "tac-storage.js",
            include_str!("../template/tac-storage.js"),
        );
        Self::write(
            root,
            "tac-client.js",
            include_str!("../template/tac-client.js"),
        );
    }

    fn stored_fields() -> serde_json::Value {
        let (relative, source) = Self::STATE_SOURCE;
        let fields = crate::behavior::StoredState::fields(Path::new(relative), source)
            .unwrap_or_else(|failure| panic!("{failure}"));
        json!(fields)
    }
}

#[tokio::test]
async fn a_dart_service_maps_through_the_converter_it_needs() {
    if !DartStreams::tools_available() {
        eprintln!("skipping: dart or node is unavailable");
        return;
    }
    let root = DartMarkerProofs::project();
    DartStreams::publish(
        root.path(),
        &DartMarkerProofs::CONVERTER_FILES,
        &DartMarkerProofs::CONVERTER_SERVICES,
        &["catalog", "card"],
    )
    .await;
    let report = DartStreams::drive(root.path(), DartMarkerProofs::CONVERTER_DRIVER);
    assert_eq!(report["before"], "", "{report}");
    assert_eq!(report["result"], "product 7: Anvil, detail, 1", "{report}");
    assert_eq!(
        report["card"],
        json!({"title": "Dune", "absent": null, "kind": "book"}),
        "{report}"
    );
}

#[tokio::test]
async fn a_dart_needs_all_service_takes_its_fields_from_the_component_properties() {
    if !DartStreams::tools_available() {
        eprintln!("skipping: dart or node is unavailable");
        return;
    }
    let root = DartMarkerProofs::project();
    DartStreams::publish(
        root.path(),
        &DartMarkerProofs::NEEDS_ALL_FILES,
        &DartMarkerProofs::NEEDS_ALL_SERVICES,
        &["pager"],
    )
    .await;
    let report = DartStreams::drive(root.path(), DartMarkerProofs::NEEDS_ALL_DRIVER);
    assert_eq!(
        report["docs"],
        json!(["/docs", 20, "/docs:20:1", "/docs:20:2"]),
        "{report}"
    );
    assert_eq!(report["shop"], json!(["/shop", 5, "/shop:5:1"]), "{report}");
}

#[tokio::test]
async fn a_dart_service_writes_its_local_state_to_page_storage_and_restores_it() {
    let stored = DartMarkerProofs::stored_fields();
    assert_eq!(stored, json!({"draft": "session", "visits": "local"}));
    if !DartStreams::tools_available() {
        eprintln!("skipping: dart or node is unavailable");
        return;
    }
    let root = DartMarkerProofs::project();
    DartStreams::publish(
        root.path(),
        &[DartMarkerProofs::STATE_SOURCE],
        &DartMarkerProofs::STATE_SERVICES,
        &["draft"],
    )
    .await;
    DartMarkerProofs::with_page_runtime(root.path());
    DartMarkerProofs::write(root.path(), "state.json", &stored.to_string());
    let report = DartStreams::drive(root.path(), DartMarkerProofs::STATE_DRIVER);
    assert_eq!(
        report["written"],
        json!({
            "saved": {"tac:service:draft:/:visits": "2"},
            "session": {"tac:service:draft:/:draft": "\"chapter one\""},
        }),
        "{report}"
    );
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
}

#[tokio::test]
async fn a_dart_retry_method_is_called_again_after_it_fails() {
    if !DartStreams::tools_available() {
        eprintln!("skipping: dart or node is unavailable");
        return;
    }
    let root = DartMarkerProofs::project();
    DartStreams::publish(
        root.path(),
        &DartMarkerProofs::RETRY_FILES,
        &DartMarkerProofs::RETRY_SERVICES,
        &["orders", "flaky"],
    )
    .await;
    DartMarkerProofs::with_page_runtime(root.path());
    let report = DartStreams::drive(root.path(), DartMarkerProofs::RETRY_DRIVER);
    assert_eq!(
        report["submitted"],
        json!({"value": {"id": "o1", "attempts": 3}}),
        "{report}"
    );
    assert_eq!(report["orderAttempts"], 3, "{report}");
    assert!(report["exhausted"]["error"].is_string(), "{report}");
    assert_eq!(report["attempts"], 3, "{report}");
    assert!(report["plain"]["error"].is_string(), "{report}");
    assert_eq!(report["plainAttempts"], 1, "{report}");
}

#[tokio::test]
async fn a_dart_in_background_listener_runs_after_the_publishing_call_returns() {
    if !DartStreams::tools_available() {
        eprintln!("skipping: dart or node is unavailable");
        return;
    }
    let root = DartMarkerProofs::project();
    DartStreams::publish(
        root.path(),
        &DartMarkerProofs::BACKGROUND_FILES,
        &DartMarkerProofs::BACKGROUND_SERVICES,
        &["journal"],
    )
    .await;
    let report = DartStreams::drive(root.path(), DartMarkerProofs::BACKGROUND_DRIVER);
    assert_eq!(
        report["listeners"],
        json!([["audit", true], ["record", false]]),
        "{report}"
    );
    assert_eq!(
        report["returned"],
        json!(["place 3", "record 3", "placed 3"]),
        "{report}"
    );
    assert_eq!(
        report["after"],
        json!(["place 3", "record 3", "placed 3", "audit 3"]),
        "{report}"
    );
    assert_eq!(report["failures"], 0, "{report}");
}
