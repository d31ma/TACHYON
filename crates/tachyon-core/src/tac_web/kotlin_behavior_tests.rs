#![allow(clippy::expect_used)]

use super::kotlin_tests::kotlin_web_ready;
use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use serde_json::json;
use std::path::Path;

struct KotlinBehaviors;

impl KotlinBehaviors {
    const FILES: [(&str, &str); 12] = [
        (
            "client/setups/ShelfSetup.kt",
            "@Setup\nclass ShelfSetup {\n    companion object {\n        \
             fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n",
        ),
        (
            "client/entities/BookEntity.kt",
            "@Entity({\"adapter\":\"indexeddb\"})\n@Table(\"books\")\nclass BookEntity {\n    @Id\n    var id: String = \"\"\n    var status: String = \"\"\n}\n",
        ),
        (
            "client/stores/BookStore.kt",
            "@Store(BookEntity::class, {\"fields\":[\"id\",\"status\"],\"defaultSize\":2,\"maxSize\":3,\"setup\":\"ShelfSetup\"})\n\
             interface BookStore : Repository<BookEntity, String> {\n    \
             fun findByStatus(status: String): Any?\n    \
             fun countByStatus(status: String): Any?\n    \
             @Query({\"operation\":\"find\",\"where\":[{\"field\":\"status\",\"operator\":\"ne\",\"argument\":0}],\"orderBy\":[{\"field\":\"id\",\"direction\":\"desc\"}]})\n    \
             fun excluding(status: String, @PageDefaults page: Map<String, Any?>): Any?\n}\n",
        ),
        (
            "client/services/ShelfService.kt",
            "@Service\n@CacheSettings({\"name\":\"shelf\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":8})\nclass ShelfService {\n    \
             private lateinit var behaviors: Behaviors.Context\n    \
             @Needs\n    private val bookStore: BookStore\n    \
             private var reads: Long = 0\n\n    \
             @OnStart\n    fun open() {\n        behaviors = Behaviors.create(\"ShelfSetup\", Behaviors.Options(\"shelf\", Behaviors.Signal()))\n    }\n\n    \
             @Cached\n    fun price(sku: String): Any? {\n        reads += 1\n        return mapOf(\"sku\" to sku, \"reads\" to reads)\n    }\n\n    \
             @UpdateCache({\"method\":\"price\"})\n    fun reprice(sku: String, reads: Long): Any? {\n        return mapOf(\"sku\" to sku, \"reads\" to reads)\n    }\n\n    \
             fun page(@PageDefaults({\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"]}) input: Any?): Any? {\n        return input\n    }\n\n    \
             fun add(id: String, status: String): Any? {\n        return behaviors.repository(\"BookEntity\").save(mapOf(\"id\" to id, \"status\" to status))\n    }\n\n    \
             fun report(): Any? {\n        return mapOf(\n            \"active\" to bookStore.findByStatus(\"active\"),\n            \"count\" to bookStore.countByStatus(\"active\"),\n            \
             \"others\" to bookStore.excluding(\"active\", mapOf()),\n            \"full\" to bookStore.excluding(\"active\", mapOf(\"size\" to 3)),\n        )\n    }\n\n    \
             fun oversized(): Any? {\n        return bookStore.excluding(\"active\", mapOf(\"size\" to 4))\n    }\n\n    \
             @MaxAtOnce({\"limit\":1,\"policy\":\"reject\"})\n    fun once(again: Boolean): Any? {\n        if (!again) return \"inside\"\n        \
             return try {\n            once(false)\n            \"entered twice\"\n        } catch (error: Behaviors.Failure) {\n            error.code\n        }\n    }\n}\n",
        ),
        (
            "client/models/OrderInput.kt",
            "@Checked\nclass OrderInput {\n    @NotEmpty\n    @Length({\"min\":1,\"max\":80})\n    var name: Any? = null\n    @Required\n    @Min(1)\n    @Max(9)\n    var level: Any? = null\n}\n",
        ),
        (
            "client/services/OrderService.kt",
            "import models.OrderInput\n\n@Service\nclass OrderService {\n    fun accept(@Check(OrderInput::class) order: Any?): Any? {\n        return order\n    }\n}\n",
        ),
        (
            "client/models/ShelfChanged.kt",
            "@Topic\ndata class ShelfChanged(val count: Int)\n",
        ),
        (
            "client/services/ListenerService.kt",
            "import models.ShelfChanged\n\n@Service\nclass ListenerService {\n    var heard: List<Int> = emptyList()\n\n    \
             @OnEvent\n    fun changed(value: ShelfChanged) {\n        heard = heard + value.count\n    }\n}\n",
        ),
        (
            "client/remotes/ParcelClient.kt",
            "@HTTPClient(\"/api\")\ninterface ParcelClient {\n    @PostRequest(\"/parcels\")\n    fun send(@RequestBody parcel: Any?): Any?\n}\n",
        ),
        (
            "client/setups/ParcelSetup.kt",
            "@Setup\nclass ParcelSetup {\n    companion object {\n        \
             fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration(\n            \
             http = mapOf(\"parcel\" to Behaviors.HttpAdapter { request, _ ->\n                \
             mapOf(\"method\" to request[\"method\"], \"path\" to request[\"path\"], \"body\" to request[\"body\"])\n            }))\n    }\n}\n",
        ),
        (
            "client/services/PostService.kt",
            "@Service\nclass PostService {\n    @Needs\n    private val parcels: ParcelClient\n\n    \
             fun post(sku: String): Any? {\n        return parcels.send(mapOf(\"sku\" to sku, \"count\" to 2))\n    }\n}\n",
        ),
        (
            "client/services/QuietService.kt",
            "@Service\nclass QuietService {\n    var count: Int = 0\n}\n",
        ),
    ];

    const SERVICES: [(&str, &str); 5] = [
        ("shelf", "ShelfService"),
        ("order", "OrderService"),
        ("listener", "ListenerService"),
        ("post", "PostService"),
        ("quiet", "QuietService"),
    ];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         console.info = () => {}\n\
         globalThis.indexedDB = { databases: async () => [], open: () => ({}) }\n\
         ;(0, eval)(await readFile('./tac-storage.js', 'utf8'))\n\
         const sources = []\n\
         globalThis.EventSource = class { constructor(url) { this.url = String(url); sources.push(this) } addEventListener() {} close() {} }\n\
         const { default: ShelfService } = await import('./ShelfService.js')\n\
         const { default: OrderService } = await import('./OrderService.js')\n\
         const { default: ListenerService, __tachyon_event_plan__: listenerPlan } = await import('./ListenerService.js')\n\
         const { default: QuietService, __tachyon_event_plan__: quietPlan } = await import('./QuietService.js')\n\
         const { default: PostService } = await import('./PostService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const outcome = (run) => { try { return { value: run() } } catch (error) { return { error: String(error?.message ?? error) } } }\n\
         const shelf = new ShelfService({})\n\
         shelf.__tachyonStarted()\n\
         const first = shelf.price('anvil')\n\
         const again = shelf.price('anvil')\n\
         const written = shelf.reprice('anvil', 99)\n\
         const updated = shelf.price('anvil')\n\
         const other = shelf.price('hammer')\n\
         const pages = [shelf.page({}), shelf.page({ page: 2, sort: ['id:desc'] })]\n\
         const badPages = [outcome(() => shelf.page({ size: 101 })), outcome(() => shelf.page({ sort: ['title'] }))]\n\
         for (const [id, status] of [['b1', 'active'], ['b2', 'retired'], ['b3', 'active'], ['b4', 'draft'], ['b5', 'lost'], ['b6', 'lent']]) shelf.add(id, status)\n\
         const store = shelf.report()\n\
         const oversized = outcome(() => shelf.oversized())\n\
         const once = [shelf.once(true), shelf.once(false)]\n\
         const order = new OrderService({})\n\
         const accepted = outcome(() => order.accept({ name: 'Tea', level: 2 }))\n\
         const invalid = [{ name: '', level: 2 }, { name: '   ', level: 2 }, { name: 'x'.repeat(81), level: 2 }, { name: 'Tea' }, { name: 'Tea', level: 0 }, { name: 'Tea', level: 10 }]\n\
         const refused = invalid.map((value) => outcome(() => order.accept(value)))\n\
         const published = []\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => {} })\n\
         bus.attach({ receive: (value) => published.push(value) }, { listeners: [{ method: 'receive', event: 'ShelfChanged' }] }, {})\n\
         const listener = new ListenerService({}), quiet = new QuietService({})\n\
         bus.attach(listener, listenerPlan.types[0], { applicationEventPublisher: null })\n\
         bus.attach(quiet, quietPlan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(listener)\n\
         await bus.initialize(quiet)\n\
         for (const source of sources) source.onmessage({ data: JSON.stringify({ count: 3 }), lastEventId: '1' })\n\
         for (const source of sources) source.onmessage({ data: JSON.stringify({ count: 'many' }), lastEventId: '2' })\n\
         for (const source of sources) source.onmessage({ data: JSON.stringify({ count: 5 }), lastEventId: '3' })\n\
         const post = new PostService({})\n\
         const posted = post.post('anvil')\n\
         console.log(JSON.stringify({ first, again, written, updated, other, pages, badPages, store, oversized, once, accepted, refused,\n\
           topics: sources.map((source) => source.url), heard: listener.heard, published, posted }))\n\
         process.exit(0)\n";

    fn write(root: &Path, relative: &str, text: &str) -> std::path::PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(&path, text).expect("file");
        path
    }

    async fn publish(root: &Path) {
        for (relative, text) in Self::FILES {
            Self::write(root, relative, text);
        }
        let services: Vec<_> = Self::SERVICES
            .iter()
            .map(|(instance, class)| {
                (
                    String::from(*instance),
                    String::from(*class),
                    WebLanguage::Kotlin,
                    root.join(format!("client/services/{class}.kt")),
                )
            })
            .collect();
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
            "tac-storage.js",
            include_str!("../template/tac-storage.js"),
        );
    }

    fn drive(root: &Path) -> serde_json::Value {
        std::fs::write(root.join("drive.mjs"), Self::DRIVER).expect("driver");
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

    fn refused_with(outcome: &serde_json::Value, code: &str) -> bool {
        outcome["error"]
            .as_str()
            .is_some_and(|message| message.ends_with(code))
    }

    fn assert_cache_pages_store_and_limits(report: &serde_json::Value) {
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
            Self::refused_with(&report["badPages"][0], "pagination"),
            "{report}"
        );
        assert!(
            Self::refused_with(&report["badPages"][1], "pagination"),
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
            Self::refused_with(&report["oversized"], "pagination"),
            "{report}"
        );

        assert_eq!(report["once"], json!(["capacity", "inside"]), "{report}");
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
async fn kotlin_behavior_markers_take_effect_on_the_page() {
    if !kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, TAC_KOTLIN_WASM_STDLIB or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    KotlinBehaviors::publish(root.path()).await;
    let report = KotlinBehaviors::drive(root.path());

    KotlinBehaviors::assert_cache_pages_store_and_limits(&report);
    KotlinBehaviors::assert_checks_topics_and_body(&report);
}
