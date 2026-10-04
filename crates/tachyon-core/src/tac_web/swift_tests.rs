#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use std::path::{Path, PathBuf};
use std::process::Command;

struct SwiftFixtures;

impl SwiftFixtures {
    const COUNTER_SERVICE: &str = "@Service\n@Lifetime({\"value\":\"prototype\"})\nfinal class CounterService {\n    var count: Int = 0\n    var label: String = \"\"\n    var price: Double = 1.5\n    let runtime = \"Swift\"\n\n    \
         struct CounterFailure: Error, CustomStringConvertible {\n        let description: String\n    }\n\n    \
         @OnStart\n    func start() {\n        label += \" clicks\"\n    }\n\n    \
         func increment() -> Int {\n        count += 1\n        return count\n    }\n\n    \
         func doubled() -> Int { count * 2 }\n\n    \
         func add(_ by: Int, why: String) -> String {\n        count += by\n        return why + \" \" + String(count)\n    }\n\n    \
         func scale(by factor: Double) -> Double {\n        price *= factor\n        return price\n    }\n\n    \
         func fail(_ why: String) throws -> Int {\n        throw CounterFailure(description: \"counter \" + why)\n    }\n}\n";

    const COUNTER_DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const { default: CounterService } = await import('./CounterService.js')\n\
         const one = new CounterService({}), two = new CounterService({})\n\
         one.__tachyonStarted(); two.__tachyonStarted()\n\
         one.increment(); one.increment(); two.increment()\n\
         const doubled = one.doubled()\n\
         one.count = 40\n\
         const after = one.increment()\n\
         const added = one.add(new Event('click'), 9, 'now')\n\
         one.price = 2\n\
         const scaled = one.scale(1.25)\n\
         let failed = ''\n\
         try { one.fail('broke') } catch (error) { failed = error.message }\n\
         let bigStart = '', bigCall = '', largeStart = ''\n\
         try { new CounterService({ label: 'x'.repeat(1100000) }) } catch (error) { bigStart = error.message }\n\
         try { new CounterService({ label: 'x'.repeat(921600) }).__tachyonDispose() } catch (error) { largeStart = error.message }\n\
         try { one.add(new Event('click'), 1, 'x'.repeat(70000)) } catch (error) { bigCall = error.message }\n\
         one.__tachyonDispose()\n\
         let stale = false\n\
         try { one.count } catch { stale = true }\n\
         console.log(JSON.stringify({ doubled, after, other: two.count, label: two.label, added, scaled, runtime: two.runtime, failed, stale, keys: Object.keys(two).sort(), bigStart, bigCall, largeStart }))\n\
         process.exit(0)\n";
    const LIBRARY_FILES: [(&str, &str); 7] = [
        (
            "client/models/Author.swift",
            "struct Author {\n    let name: String\n}\n",
        ),
        (
            "client/models/Size.swift",
            "enum Size: String {\n    case short, long\n}\n",
        ),
        (
            "client/models/Guide.swift",
            "struct Guide {\n    let title: String\n    var pages: Int\n    let rating: Double\n    let draft: Bool\n    let tags: [String]\n    let note: String?\n    let size: Size\n    let author: Author\n}\n",
        ),
        (
            "client/models/GuideSaved.swift",
            "struct GuideSaved {\n    let title: String\n    let pages: Int\n}\n",
        ),
        (
            "client/models/GuideEcho.swift",
            "struct GuideEcho {\n    let pages: Int\n}\n",
        ),
        (
            "client/models/ShelfChanged.swift",
            "@Topic\nstruct ShelfChanged {\n    let count: Int\n}\n",
        ),
        (
            "client/services/LibraryService.swift",
            "@Service\nfinal class LibraryService {\n    var latest: Guide? = nil\n    var heard: [Int] = []\n    let applicationEventPublisher: ApplicationEventPublisher\n\n    \
             func longer(_ guide: Guide, by extra: Int) -> Guide {\n        var copy = guide\n        copy.pages += extra\n        latest = copy\n        \
             applicationEventPublisher.publishEvent(GuideSaved(title: copy.title, pages: copy.pages))\n        return copy\n    }\n\n    \
             func shelf() -> [Guide] {\n        latest.map { [$0] } ?? []\n    }\n\n    \
             @OnEvent({\"condition\":\"value.pages > 10\"})\n    func saved(_ value: GuideSaved) -> GuideEcho {\n        heard.append(value.pages)\n        return GuideEcho(pages: value.pages * 10)\n    }\n\n    \
             @OnEvent\n    func shelfChanged(_ value: ShelfChanged) {\n        heard.append(value.count)\n    }\n}\n",
        ),
    ];

    const LIBRARY_DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const sources = []\n\
         globalThis.EventSource = class { constructor(url) { this.url = url; sources.push(this) } addEventListener() {} close() {} }\n\
         const { default: LibraryService, __tachyon_event_plan__: plan } = await import('./LibraryService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const events = []\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         for (const event of ['GuideSaved', 'GuideEcho']) bus.attach({ receive: (value) => events.push([event, value]) }, { listeners: [{ method: 'receive', event, order: -100 }] }, {})\n\
         const library = new LibraryService({})\n\
         bus.attach(library, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(library)\n\
         const guide = { title: 'Swift', pages: 8, rating: 4.5, draft: false, tags: ['wasm'], note: null, size: 'long', author: { name: 'Ann' } }\n\
         const small = library.longer(guide, 1)\n\
         const large = library.longer(guide, 4)\n\
         for (const source of sources) source.onmessage({ data: JSON.stringify({ count: 3 }), lastEventId: '1' })\n\
         let refused = ''\n\
         try { library.longer({ title: 'broken' }, 1) } catch (error) { refused = error.message }\n\
         console.log(JSON.stringify({ small, large, latest: library.latest, shelf: library.shelf(), heard: library.heard, events, topics: sources.map((source) => source.url), refused }))\n\
         process.exit(0)\n";

    const STORAGE_FILES: [(&str, &str); 3] = [
        (
            "client/setups/BrowserSetup.swift",
            "@Setup\nfinal class BrowserSetup {\n    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {\n        BehaviorAdapters()\n    }\n}\n",
        ),
        (
            "client/entities/NoteEntity.swift",
            "@Entity({\"adapter\":\"indexeddb\"})\n@Table(\"notes\")\nfinal class NoteEntity {\n    @Id\n    var id: String = \"\"\n    var text: String = \"\"\n}\n",
        ),
        (
            "client/services/NotesService.swift",
            "@Service\n@CacheSettings({\"name\":\"notes\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":8})\nfinal class NotesService {\n    \
             private var behaviors: BehaviorContext!\n    private var reads: Int = 0\n\n    \
             @OnStart\n    func open() {\n        behaviors = try? Behaviors.create(BrowserSetup.self, partition: \"notes\", signal: BehaviorSignal())\n    }\n\n    \
             @Cached\n    func find(_ key: BehaviorJSON) async throws -> BehaviorJSON {\n        reads += 1\n        return .object([\"key\": key, \"reads\": .number(Double(reads))])\n    }\n\n    \
             @ClearCache({\"method\":\"find\"})\n    func forget(_ key: BehaviorJSON) async throws -> BehaviorJSON {\n        key\n    }\n\n    \
             func save(_ id: String, _ text: String) async throws -> BehaviorJSON {\n        try await notes().save(.object([\"id\": .string(id), \"text\": .string(text)]))\n    }\n\n    \
             @AllOrNothing({\"adapter\":\"indexeddb\"})\n    func saveThenRefuse(_ id: String) async throws -> BehaviorJSON {\n        \
             _ = try await save(id, \"draft\")\n        throw BehaviorError(\"the note was refused\")\n    }\n\n    \
             func list() async throws -> BehaviorJSON {\n        try await notes().find()\n    }\n\n    \
             private func notes() throws -> BehaviorRepository {\n        try behaviors.repository(NoteEntity.self)\n    }\n}\n",
        ),
    ];

    const STORAGE_DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         console.info = () => {}\n\
         globalThis.indexedDB = { databases: async () => [], open: () => ({}) }\n\
         ;(0, eval)(await readFile('./tac-storage.js', 'utf8'))\n\
         const { default: NotesService } = await import('./NotesService.js')\n\
         const notes = new NotesService({})\n\
         notes.__tachyonStarted()\n\
         const first = await notes.find('a')\n\
         const second = await notes.find('a')\n\
         await notes.forget('a')\n\
         const third = await notes.find('a')\n\
         await notes.save('n1', 'hello')\n\
         let refused = ''\n\
         try { await notes.saveThenRefuse('n2') } catch (error) { refused = error.message }\n\
         console.log(JSON.stringify({ first, second, third, refused, listed: await notes.list() }))\n\
         process.exit(0)\n";
}

struct SwiftBehaviorFixtures;

impl SwiftBehaviorFixtures {
    const FILES: [(&str, &str); 5] = [
        (
            "client/setups/OrdersSetup.swift",
            r"@Setup
final class OrdersSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        var adapters = BehaviorAdapters()
        adapters.authorize = { _, _ in true }
        return adapters
    }
}
",
        ),
        (
            "client/services/OrdersService.swift",
            r#"@Service
final class OrdersService {
    struct GatewayUnavailable: Error {}

    var behaviors: BehaviorContext!
    private static let firstWorkingAttempt = 3

    var attempts: Int = 0

    @OnStart
    func open() {
        behaviors = Self.openOrders()
    }

    @Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
    func submit(_ id: BehaviorJSON) async throws -> BehaviorJSON {
        attempts += 1
        try failFirstTwoAttempts()
        return reportOf(id)
    }

    @MaxAtOnce({"limit":2,"policy":"block"})
    func report(_ id: BehaviorJSON) async throws -> BehaviorJSON {
        return reportOf(id)
    }

    private func reportOf(_ id: BehaviorJSON) -> BehaviorJSON {
        .object(["id": id, "attempts": .number(Double(attempts))])
    }

    private func failFirstTwoAttempts() throws {
        if attempts < Self.firstWorkingAttempt { throw GatewayUnavailable() }
    }

    private static func openOrders() -> BehaviorContext? {
        try? Behaviors.create(OrdersSetup.self, partition: "orders", signal: BehaviorSignal())
    }
}
"#,
        ),
        (
            "client/setups/CatalogSetup.swift",
            r#"@Setup
@Settings({"prefix":"TAC_CATALOG_"})
@WhenProfile("public")
@WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
final class CatalogSetup {
    @Min(1)
    @Max(100)
    var pageSize: Int = 20
    var cacheEnabled: Bool = false
    var title: String = "Catalog"
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters { BehaviorAdapters() }
}
"#,
        ),
        (
            "client/setups/PublicValuesSetup.swift",
            r#"@Setup
final class PublicValuesSetup {
    @Setting("${tac.catalog.title:Catalog}")
    var title: String = ""
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters { BehaviorAdapters() }
}
"#,
        ),
        (
            "client/services/SettingsService.swift",
            r#"@Service
final class SettingsService {
    var title: String = "Catalog"
    var pageSize: Int = 20

    init() {
        (title, pageSize) = Self.settingsOrFallback()
    }

    private static func settingsOrFallback() -> (String, Int) {
        do {
            return try readSettings()
        } catch {
            return ("Configuration unavailable", 20)
        }
    }

    private static func readSettings() throws -> (String, Int) {
        let published = try BehaviorJSON.parse(
            "{\"tac\":{\"catalog\":{\"title\":\"Catalog\",\"page\":{\"size\":20},\"cache\":{\"enabled\":true}}}}")
        let catalog = try Behaviors.properties(CatalogSetup.self, published)
        let values = try Behaviors.values(PublicValuesSetup.self, published)
        let context = try Behaviors.create(
            CatalogSetup.self, partition: "public-catalog", signal: BehaviorSignal(),
            profiles: ["public"], properties: published)
        context.dispose()
        return (try values.get("title").text, try catalog.get("pageSize").integer)
    }
}
"#,
        ),
    ];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         let polls = 0\n\
         const instantiate = WebAssembly.instantiate\n\
         WebAssembly.instantiate = (module, imports) => {\n\
         \x20 const wasi = imports?.wasi_snapshot_preview1\n\
         \x20 const poll = wasi?.poll_oneoff\n\
         \x20 if (poll) wasi.poll_oneoff = (...values) => { polls += 1; return poll(...values) }\n\
         \x20 return instantiate(module, imports)\n\
         }\n\
         const { default: OrdersService } = await import('./OrdersService.js')\n\
         const { default: SettingsService } = await import('./SettingsService.js')\n\
         const orders = new OrdersService({})\n\
         orders.__tachyonStarted()\n\
         const started = performance.now()\n\
         const submitted = await orders.submit('a')\n\
         const waited = performance.now() - started\n\
         const reported = await orders.report('b')\n\
         const settings = new SettingsService({})\n\
         console.log(JSON.stringify({ submitted, waited, polls, reported, title: settings.title, pageSize: settings.pageSize }))\n\
         process.exit(0)\n";
}

pub(crate) fn swift_web_ready() -> bool {
    let sdk = std::env::var_os("TAC_SWIFT_WASM_SDK").map(PathBuf::from);
    let has_sdk = sdk.is_some_and(|path| path.join("WASI.sdk").is_dir());
    let compiler = std::env::var_os("TAC_SWIFT_WASM_COMPILER").unwrap_or_else(|| "swiftc".into());
    let runs = |program: &std::ffi::OsStr, flag: &str| {
        Command::new(program)
            .arg(flag)
            .output()
            .is_ok_and(|output| output.status.success())
    };
    has_sdk && runs(&compiler, "--version") && runs("node".as_ref(), "--version")
}

fn run_driver(root: &Path, script: &str) -> serde_json::Value {
    std::fs::write(root.join("drive.mjs"), script).expect("driver");
    let output = Command::new("node")
        .arg("drive.mjs")
        .current_dir(root)
        .output()
        .expect("node");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_str(stdout.trim()).expect("report")
}

async fn build_counter(root: &Path, profile: crate::BuildProfile) -> Vec<String> {
    let source = root.join("client/services/CounterService.swift");
    std::fs::create_dir_all(source.parent().expect("parent")).expect("folder");
    std::fs::write(&source, SwiftFixtures::COUNTER_SERVICE).expect("service");
    let services = [(
        String::from("counter"),
        String::from("CounterService"),
        WebLanguage::Swift,
        source,
    )];
    let built = build_all(root, &services, &BuildJobs::new(1), profile)
        .await
        .expect("the Swift service builds");
    for (name, bytes) in built.files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, bytes).expect("publish");
    }
    built.notes
}

#[tokio::test]
async fn a_swift_service_runs_in_the_page_as_webassembly() {
    if !swift_web_ready() {
        eprintln!("skipping: the Swift SDK for WebAssembly, swiftc or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let notes = build_counter(root.path(), crate::BuildProfile::Development).await;
    assert!(notes.is_empty(), "{notes:?}");
    assert!(root.path().join("swift/services.wasm").is_file());
    assert!(root.path().join("tachyon-swift.js").is_file());

    let counter = run_driver(root.path(), SwiftFixtures::COUNTER_DRIVER);
    assert_eq!(
        counter,
        serde_json::json!({
            "doubled": 4,
            "after": 41,
            "other": 1,
            "label": " clicks",
            "added": "now 50",
            "scaled": 2.5,
            "runtime": "Swift",
            "failed": "Web service counter: counter broke",
            "stale": true,
            "keys": ["count", "label", "price", "runtime"],
            "bigStart": "Web service counter: The request is 1100115 bytes. The limit is 1048576 bytes. Pass less data to the service.",
            "bigCall": "Web service counter: The request to add is 70121 bytes. The limit is 65536 bytes. Pass less data to the service.",
            "largeStart": "",
        })
    );
}

#[tokio::test]
async fn a_swift_release_build_is_made_smaller_by_binaryen() {
    if !swift_web_ready() || !crate::wasm_optimizer::WasmOptimizer::is_installed() {
        eprintln!(
            "skipping: the Swift SDK for WebAssembly, swiftc, node or wasm-opt is unavailable"
        );
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let notes = build_counter(root.path(), crate::BuildProfile::Release).await;
    assert_eq!(
        notes,
        ["Swift: WebAssembly release build — optimized by Binaryen"]
    );
    let counter = run_driver(root.path(), SwiftFixtures::COUNTER_DRIVER);
    assert_eq!(counter["after"], 41);
}

fn write(root: &Path, relative: &str, text: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
    std::fs::write(&path, text).expect("file");
    path
}

async fn publish(root: &Path, instance: &str, class: &str) {
    let services = [(
        String::from(instance),
        String::from(class),
        WebLanguage::Swift,
        root.join(format!("client/services/{class}.swift")),
    )];
    let built = build_all(
        root,
        &services,
        &BuildJobs::new(1),
        crate::BuildProfile::Development,
    )
    .await
    .expect("the Swift service builds");
    for (name, bytes) in built.files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, bytes).expect("publish");
    }
}

#[tokio::test]
async fn a_swift_service_exchanges_models_and_events_with_the_page() {
    if !swift_web_ready() {
        eprintln!("skipping: the Swift SDK for WebAssembly, swiftc or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in SwiftFixtures::LIBRARY_FILES {
        write(root.path(), relative, text);
    }
    write(
        root.path(),
        "application-events.mjs",
        concat!(
            include_str!("../template/application-events.js"),
            "\nexport default TacApplicationEvents\n"
        ),
    );
    publish(root.path(), "library", "LibraryService").await;
    let report = run_driver(root.path(), SwiftFixtures::LIBRARY_DRIVER);
    let guide = |pages: i64| serde_json::json!({"title": "Swift", "pages": pages, "rating": 4.5, "draft": false, "tags": ["wasm"], "size": "long", "author": {"name": "Ann"}});
    assert_eq!(report["small"], guide(9));
    assert_eq!(report["large"], guide(12));
    assert_eq!(report["latest"], guide(12));
    assert_eq!(report["shelf"], serde_json::json!([guide(12)]));
    assert_eq!(report["heard"], serde_json::json!([12, 3]));
    assert_eq!(
        report["events"],
        serde_json::json!([
            ["GuideSaved", {"title": "Swift", "pages": 9}],
            ["GuideSaved", {"title": "Swift", "pages": 12}],
            ["GuideEcho", {"pages": 120}],
        ])
    );
    assert_eq!(
        report["topics"],
        serde_json::json!(["/.tachyon/topics/shelf-changed"])
    );
    assert_eq!(
        report["refused"],
        "Web service library: A service argument or field value has the wrong type."
    );
}

#[tokio::test]
async fn a_swift_service_awaits_the_builtin_cache_and_store() {
    if !swift_web_ready() {
        eprintln!("skipping: the Swift SDK for WebAssembly, swiftc or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in SwiftFixtures::STORAGE_FILES {
        write(root.path(), relative, text);
    }
    write(
        root.path(),
        "tac-storage.js",
        include_str!("../template/tac-storage.js"),
    );
    publish(root.path(), "notes", "NotesService").await;
    let report = run_driver(root.path(), SwiftFixtures::STORAGE_DRIVER);
    assert_eq!(report["first"], serde_json::json!({"key": "a", "reads": 1}));
    assert_eq!(report["second"], report["first"]);
    assert_eq!(report["third"], serde_json::json!({"key": "a", "reads": 2}));
    assert!(
        report["refused"]
            .as_str()
            .is_some_and(|message| message.contains("the note was refused")),
        "{report}"
    );
    assert_eq!(
        report["listed"],
        serde_json::json!([{"id": "n1", "text": "hello"}])
    );
}

#[tokio::test]
async fn a_swift_service_retries_after_a_delay_and_binds_settings_as_webassembly() {
    if !swift_web_ready() {
        eprintln!("skipping: the Swift SDK for WebAssembly, swiftc or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in SwiftBehaviorFixtures::FILES {
        write(root.path(), relative, text);
    }
    let services =
        [("orders", "OrdersService"), ("settings", "SettingsService")].map(|(instance, class)| {
            (
                String::from(instance),
                String::from(class),
                WebLanguage::Swift,
                root.path().join(format!("client/services/{class}.swift")),
            )
        });
    let built = build_all(
        root.path(),
        &services,
        &BuildJobs::new(1),
        crate::BuildProfile::Development,
    )
    .await
    .expect("the Swift services build");
    for (name, bytes) in built.files {
        let path = root.path().join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, bytes).expect("publish");
    }
    let report = run_driver(root.path(), SwiftBehaviorFixtures::DRIVER);
    assert_eq!(
        report["submitted"],
        serde_json::json!({"id": "a", "attempts": 3})
    );
    assert_eq!(
        report["reported"],
        serde_json::json!({"id": "b", "attempts": 3})
    );
    assert!(
        report["waited"]
            .as_f64()
            .is_some_and(|waited| waited >= 30.0),
        "{report}"
    );
    assert_eq!(report["polls"], 2, "{report}");
    assert_eq!(report["title"], "Catalog");
    assert_eq!(report["pageSize"], 20);
}
