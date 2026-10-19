#![allow(clippy::expect_used)]

use crate::node_driver::NodeDriver;
use serde_json::json;
use std::path::Path;
use std::process::Command;

struct DartBehaviors;

impl DartBehaviors {
    const FILES: [(&str, &str); 15] = [
        (
            "client/setups/ShelfSetup.dart",
            "@Setup\nclass ShelfSetup {\n  static BehaviorAdapters configure(BehaviorExecution execution) {\n    return BehaviorAdapters();\n  }\n}\n",
        ),
        (
            "client/entities/BookEntity.dart",
            "@Entity({\"adapter\":\"indexeddb\"})\n@Table(\"books\")\nclass BookEntity {\n  @Id\n  String id = '';\n  String status = '';\n}\n",
        ),
        (
            "client/stores/BookStore.dart",
            "@Store(BookEntity, {\"fields\":[\"id\",\"status\"],\"defaultSize\":2,\"maxSize\":3,\"setup\":\"ShelfSetup\"})\n\
             abstract interface class BookStore implements Repository<BookEntity, String> {\n  \
             Future<Object?> findByStatus(String status);\n\n  \
             Future<Object?> countByStatus(String status);\n\n  \
             @Query({\"operation\":\"find\",\"where\":[{\"field\":\"status\",\"operator\":\"ne\",\"argument\":0}],\"orderBy\":[{\"field\":\"id\",\"direction\":\"desc\"}]})\n  \
             Future<Object?> excluding(String status, @PageDefaults Map<String, Object?> page);\n}\n",
        ),
        (
            "client/services/ShelfService.dart",
            "@Service\n@CacheSettings({\"name\":\"shelf\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":8})\nclass ShelfService {\n  \
             @Needs\n  final BookStore bookStore;\n  late BehaviorContext behaviors;\n  int reads = 0;\n\n  \
             @OnStart\n  void open() {\n    behaviors = Behaviors.create(ShelfSetup, partition: 'shelf', signal: BehaviorSignal());\n  }\n\n  \
             @Cached\n  Future<Object?> price(String sku) async {\n    reads += 1;\n    return {'sku': sku, 'reads': reads};\n  }\n\n  \
             @UpdateCache({\"method\":\"price\"})\n  Future<Object?> reprice(String sku, int reads) async {\n    return {'sku': sku, 'reads': reads};\n  }\n\n  \
             Future<Object?> page(@PageDefaults({\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"]}) Map<String, Object?> input) async {\n    return input;\n  }\n\n  \
             Future<Object?> add(String id, String status) {\n    return behaviors.repository(BookEntity).save({'id': id, 'status': status});\n  }\n\n  \
             Future<Object?> report() async {\n    return {\n      'active': await bookStore.findByStatus('active'),\n      'count': await bookStore.countByStatus('active'),\n      \
             'others': await bookStore.excluding('active', {}),\n      'full': await bookStore.excluding('active', {'size': 3}),\n    };\n  }\n\n  \
             Future<Object?> oversized() {\n    return bookStore.excluding('active', {'size': 4});\n  }\n}\n",
        ),
        (
            "client/services/GateService.dart",
            "@Service\nclass GateService {\n  late BehaviorContext behaviors;\n  List<String> steps = [];\n\n  \
             @OnStart\n  void open() {\n    behaviors = Behaviors.create(ShelfSetup, partition: 'gate', signal: BehaviorSignal());\n  }\n\n  \
             @MaxAtOnce({\"limit\":1,\"policy\":\"reject\"})\n  Future<Object?> once(String name) async {\n    await Future<void>.delayed(const Duration(milliseconds: 20));\n    return name;\n  }\n\n  \
             @MaxAtOnce({\"limit\":1,\"policy\":\"block\"})\n  Future<Object?> queued(String name) async {\n    steps.add('start $name');\n    await Future<void>.delayed(const Duration(milliseconds: 20));\n    steps.add('end $name');\n    return name;\n  }\n\n  \
             Future<Object?> rejected() async {\n    final first = once('first');\n    String second;\n    try {\n      second = '${await once('second')}';\n    } on BehaviorError catch (error) {\n      second = error.code;\n    }\n    \
             return [await first, second, await once('third')];\n  }\n\n  \
             Future<Object?> blocked() async {\n    final answers = await Future.wait([queued('a'), queued('b')]);\n    return {'answers': answers, 'steps': steps};\n  }\n}\n",
        ),
        (
            "client/setups/CatalogSetup.dart",
            "@Setup\n@Settings({\"prefix\":\"TAC_CATALOG_\"})\n@WhenProfile(\"public\")\n@WhenSetting({\"name\":\"TAC_CATALOG_CACHE_ENABLED\",\"havingValue\":\"true\",\"matchIfMissing\":false})\nclass CatalogSetup {\n  \
             @Min(1)\n  @Max(100)\n  int pageSize = 20;\n  bool cacheEnabled = false;\n  String title = 'Catalog';\n  \
             static BehaviorAdapters configure(BehaviorExecution execution) {\n    return BehaviorAdapters();\n  }\n}\n",
        ),
        (
            "client/models/PublicValues.dart",
            "class PublicValues {\n  @Setting(\"${tac.catalog.title:Catalog}\")\n  String title = '';\n}\n",
        ),
        (
            "client/services/SettingsService.dart",
            "import 'models/PublicValues.dart';\n\n@Service\nclass SettingsService {\n  \
             Map<String, Object?> _published(int size, String enabled) {\n    \
             final catalog = <String, Object?>{'title': 'Shop', 'page': {'size': size}};\n    \
             if (enabled.isNotEmpty) catalog['cache'] = {'enabled': enabled == 'true'};\n    \
             return {'tac': {'catalog': catalog}};\n  }\n\n  \
             Object? catalog(int size) {\n    try {\n      return Behaviors.properties(CatalogSetup, _published(size, 'true'));\n    } on BehaviorError catch (error) {\n      return error.code;\n    }\n  }\n\n  \
             Object? values(String title) {\n    final application = title.isEmpty\n        ? <String, Object?>{'tac': {'other': {'size': 1}}}\n        : <String, Object?>{'tac': {'catalog': {'title': title}}};\n    \
             return Behaviors.values(PublicValues, application);\n  }\n\n  \
             String open(String profile, String enabled) {\n    try {\n      final profiles = profile.isEmpty ? <String>[] : [profile];\n      \
             Behaviors.create(CatalogSetup,\n          partition: 'catalog', signal: BehaviorSignal(), profiles: profiles, properties: _published(20, enabled))\n        .dispose();\n      \
             return 'open';\n    } on BehaviorError catch (error) {\n      return error.code;\n    }\n  }\n}\n",
        ),
        (
            "client/models/OrderInput.dart",
            "@Checked\nclass OrderInput {\n  @NotEmpty\n  @Length({\"min\":1,\"max\":80})\n  Object? name;\n  @Required\n  @Min(1)\n  @Max(9)\n  Object? level;\n}\n",
        ),
        (
            "client/services/OrderService.dart",
            "import 'models/OrderInput.dart';\n\n@Service\nclass OrderService {\n  Object? accept(@Check(OrderInput) Object? order) {\n    return order;\n  }\n}\n",
        ),
        (
            "client/models/ShelfChanged.dart",
            "@Topic\nclass ShelfChanged {\n  final int count;\n\n  const ShelfChanged(this.count);\n}\n",
        ),
        (
            "client/services/ListenerService.dart",
            "import 'models/ShelfChanged.dart';\n\n@Service\nclass ListenerService {\n  List<int> heard = [];\n\n  \
             @OnEvent\n  void changed(ShelfChanged value) {\n    heard = [...heard, value.count];\n  }\n}\n",
        ),
        (
            "client/remotes/ParcelClient.dart",
            "@HTTPClient(\"/api\")\nabstract interface class ParcelClient {\n  @PostRequest(\"/parcels\")\n  Future<Object?> send(@RequestBody Object? parcel);\n}\n",
        ),
        (
            "client/setups/ParcelSetup.dart",
            "@Setup\nclass ParcelSetup {\n  static BehaviorAdapters configure(BehaviorExecution execution) {\n    \
             return BehaviorAdapters(http: {'parcel': BehaviorHttpAdapter(exchange: echo)});\n  }\n\n  \
             static Future<Object?> echo(Object? request, BehaviorExecution execution) async {\n    final sent = request as Map;\n    \
             return {'method': sent['method'], 'path': sent['path'], 'body': sent['body']};\n  }\n}\n",
        ),
        (
            "client/services/PostService.dart",
            "@Service\nclass PostService {\n  @Needs\n  final ParcelClient parcelClient;\n\n  \
             Future<Object?> post(String sku) {\n    return parcelClient.send({'sku': sku, 'count': 2});\n  }\n}\n",
        ),
    ];

    const SERVICES: [(&str, &str); 6] = [
        ("ShelfService", "client/services/ShelfService.dart"),
        ("GateService", "client/services/GateService.dart"),
        ("SettingsService", "client/services/SettingsService.dart"),
        ("OrderService", "client/services/OrderService.dart"),
        ("ListenerService", "client/services/ListenerService.dart"),
        ("PostService", "client/services/PostService.dart"),
    ];

    const INSTANCES: [&str; 6] = ["shelf", "gate", "settings", "order", "listener", "post"];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         console.info = () => {}\n\
         globalThis.indexedDB = { databases: async () => [], open: () => ({}) }\n\
         ;(0, eval)(await readFile('./tac-storage.js', 'utf8'))\n\
         const sources = []\n\
         globalThis.EventSource = class { constructor(url) { this.url = String(url); sources.push(this) } addEventListener() {} close() {} }\n\
         const { default: ShelfService, __tachyon_event_plan__: shelfPlan } = await import('./ShelfService.js')\n\
         const { default: GateService, __tachyon_event_plan__: gatePlan } = await import('./GateService.js')\n\
         const { default: SettingsService } = await import('./SettingsService.js')\n\
         const { default: OrderService } = await import('./OrderService.js')\n\
         const { default: ListenerService, __tachyon_event_plan__: listenerPlan } = await import('./ListenerService.js')\n\
         const { default: PostService } = await import('./PostService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.js')\n\
         const outcome = (run) => Promise.resolve().then(run).then((value) => ({ value }), (error) => ({ error: String(error?.message ?? error) }))\n\
         const shelf = new ShelfService({})\n\
         shelf[shelfPlan.types[0].post_construct]()\n\
         const first = await shelf.price('anvil')\n\
         const again = await shelf.price('anvil')\n\
         const written = await shelf.reprice('anvil', 99)\n\
         const updated = await shelf.price('anvil')\n\
         const other = await shelf.price('hammer')\n\
         const pages = [await shelf.page({}), await shelf.page({ page: 2, sort: ['id:desc'] })]\n\
         const badPages = [await outcome(() => shelf.page({ size: 101 })), await outcome(() => shelf.page({ sort: ['title'] }))]\n\
         for (const [id, status] of [['b1', 'active'], ['b2', 'retired'], ['b3', 'active'], ['b4', 'draft'], ['b5', 'lost'], ['b6', 'lent']]) await shelf.add(id, status)\n\
         const store = await shelf.report()\n\
         const oversized = await outcome(() => shelf.oversized())\n\
         const gate = new GateService({})\n\
         gate[gatePlan.types[0].post_construct]()\n\
         const rejected = await gate.rejected()\n\
         const blocked = await gate.blocked()\n\
         const settings = new SettingsService({})\n\
         const bound = settings.catalog(35)\n\
         const bounds = [settings.catalog(0), settings.catalog(101), settings.catalog(100).pageSize]\n\
         const values = [settings.values('Shop'), settings.values('')]\n\
         const gates = { open: settings.open('public', 'true'), noProfile: settings.open('', 'true'), otherProfile: settings.open('private', 'true'), off: settings.open('public', 'false'), missing: settings.open('public', '') }\n\
         const order = new OrderService({})\n\
         const accepted = await outcome(() => order.accept({ name: 'Tea', level: 2 }))\n\
         const invalid = [{ name: '', level: 2 }, { name: '   ', level: 2 }, { name: 'x'.repeat(81), level: 2 }, { name: 'Tea' }, { name: 'Tea', level: 0 }, { name: 'Tea', level: 10 }]\n\
         const refused = []\n\
         for (const value of invalid) refused.push(await outcome(() => order.accept(value)))\n\
         const published = []\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => {} })\n\
         bus.attach({ receive: (value) => published.push(value) }, { listeners: [{ method: 'receive', event: 'ShelfChanged' }] }, {})\n\
         const listener = new ListenerService({})\n\
         bus.attach(listener, listenerPlan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(listener)\n\
         for (const source of sources) source.onmessage({ data: JSON.stringify({ count: 3 }), lastEventId: '1' })\n\
         for (const source of sources) source.onmessage({ data: JSON.stringify({ count: 'many' }), lastEventId: '2' })\n\
         for (const source of sources) source.onmessage({ data: JSON.stringify({ count: 5 }), lastEventId: '3' })\n\
         const post = new PostService({})\n\
         const posted = await post.post('anvil')\n\
         console.log(JSON.stringify({ first, again, written, updated, other, pages, badPages, store, oversized, rejected, blocked, bound, bounds, values, gates, accepted, refused,\n\
           topics: sources.map((source) => source.url), heard: listener.heard, published, posted }))\n\
         process.exit(0)\n";

    fn tools_available() -> bool {
        ["dart", "node"].iter().all(|tool| {
            Command::new(tool)
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success())
        })
    }

    fn write(root: &Path, relative: &str, bytes: &[u8]) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(&path, bytes).expect("file");
    }

    async fn publish(root: &Path) {
        for (relative, text) in Self::FILES {
            Self::write(root, relative, text.as_bytes());
        }
        let paths: Vec<(&str, std::path::PathBuf)> = Self::SERVICES
            .iter()
            .map(|(class, relative)| (*class, root.join(relative)))
            .collect();
        let services: Vec<(&str, &Path)> = paths
            .iter()
            .map(|(class, path)| (*class, path.as_path()))
            .collect();
        let instances = Self::INSTANCES
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        let built = super::build_all(root, &services, &instances, crate::BuildProfile::Release)
            .await
            .unwrap_or_else(|failure| panic!("{failure}"));
        for (name, bytes) in built {
            Self::write(root, &name, &bytes);
        }
        Self::write(root, "package.json", b"{\"type\":\"module\"}");
        Self::write(
            root,
            "application-events.js",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            )
            .as_bytes(),
        );
        Self::write(
            root,
            "tac-storage.js",
            include_str!("../template/tac-storage.js").as_bytes(),
        );
        Self::write(
            root,
            "fetch-files.js",
            b"import { readFile } from 'node:fs/promises'\nglobalThis.fetch = async (url) => new Response(await readFile(url))\n",
        );
    }

    fn drive(root: &Path) -> serde_json::Value {
        Self::write(root, "drive.js", Self::DRIVER.as_bytes());
        let output = NodeDriver::output(
            NodeDriver::command()
                .args(["--import", "./fetch-files.js", "drive.js"])
                .current_dir(root),
        )
        .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.trim()).expect("report")
    }

    fn refused_with(outcome: &serde_json::Value, code: &str) -> bool {
        outcome["error"]
            .as_str()
            .is_some_and(|message| message.contains(code))
    }

    fn assert_cache_pages_and_store(report: &serde_json::Value) {
        assert_eq!(
            report["first"],
            json!({"sku": "anvil", "reads": 1}),
            "{report}"
        );
        assert_eq!(report["again"], report["first"], "{report}");
        assert_eq!(
            report["written"],
            json!({"sku": "anvil", "reads": 99}),
            "{report}"
        );
        assert_eq!(
            report["updated"],
            json!({"sku": "anvil", "reads": 99}),
            "{report}"
        );
        assert_eq!(
            report["other"],
            json!({"sku": "hammer", "reads": 2}),
            "{report}"
        );

        assert_eq!(
            report["pages"],
            json!([
                {"page": 0, "size": 20, "offset": 0, "sort": []},
                {"page": 2, "size": 20, "offset": 40, "sort": ["id:desc"]},
            ]),
            "{report}"
        );
        assert!(
            Self::refused_with(&report["badPages"][0], "PAGE_INVALID"),
            "{report}"
        );
        assert!(
            Self::refused_with(&report["badPages"][1], "PAGE_SORT"),
            "{report}"
        );

        let book = |id: &str, status: &str| json!({"id": id, "status": status});
        assert_eq!(
            report["store"]["active"],
            json!([book("b1", "active"), book("b3", "active")]),
            "{report}"
        );
        assert_eq!(report["store"]["count"], 2, "{report}");
        assert_eq!(
            report["store"]["others"],
            json!([book("b6", "lent"), book("b5", "lost")]),
            "{report}"
        );
        assert_eq!(
            report["store"]["full"],
            json!([book("b6", "lent"), book("b5", "lost"), book("b4", "draft")]),
            "{report}"
        );
        assert!(
            Self::refused_with(&report["oversized"], "PAGE_INVALID"),
            "{report}"
        );
    }

    fn assert_limits_and_settings(report: &serde_json::Value) {
        assert_eq!(
            report["rejected"],
            json!(["first", "OPERATION_LIMIT", "third"]),
            "{report}"
        );
        assert_eq!(
            report["blocked"],
            json!({"answers": ["a", "b"], "steps": ["start a", "end a", "start b", "end b"]}),
            "{report}"
        );

        assert_eq!(
            report["bound"],
            json!({"pageSize": 35, "cacheEnabled": true, "title": "Shop"}),
            "{report}"
        );
        assert_eq!(
            report["bounds"],
            json!(["configuration-properties", "configuration-properties", 100]),
            "{report}"
        );
        assert_eq!(
            report["values"],
            json!([{"title": "Shop"}, {"title": "Catalog"}]),
            "{report}"
        );
        assert_eq!(
            report["gates"],
            json!({
                "open": "open",
                "noProfile": "inactive-configuration",
                "otherProfile": "inactive-configuration",
                "off": "inactive-configuration",
                "missing": "inactive-configuration",
            }),
            "{report}"
        );
    }

    fn assert_checks_topics_and_body(report: &serde_json::Value) {
        assert_eq!(
            report["accepted"],
            json!({"value": {"name": "Tea", "level": 2}}),
            "{report}"
        );
        let refused = report["refused"].as_array().expect("refused");
        assert_eq!(refused.len(), 6, "{report}");
        assert!(
            refused
                .iter()
                .all(|outcome| Self::refused_with(outcome, "validation")),
            "{report}"
        );

        assert_eq!(
            report["topics"],
            json!(["/.tachyon/topics/shelf-changed"]),
            "{report}"
        );
        assert_eq!(report["heard"], json!([3, 5]), "{report}");
        assert_eq!(
            report["published"],
            json!([{"count": 3}, {"count": 5}]),
            "{report}"
        );

        assert_eq!(
            report["posted"],
            json!({"method": "POST", "path": "/api/parcels", "body": {"sku": "anvil", "count": 2}}),
            "{report}"
        );
    }
}

#[tokio::test]
async fn dart_behavior_markers_take_effect_on_the_page() {
    if !DartBehaviors::tools_available() {
        eprintln!("skipping: dart or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    DartBehaviors::publish(root.path()).await;
    let report = DartBehaviors::drive(root.path());

    DartBehaviors::assert_cache_pages_and_store(&report);
    DartBehaviors::assert_limits_and_settings(&report);
    DartBehaviors::assert_checks_topics_and_body(&report);
}
