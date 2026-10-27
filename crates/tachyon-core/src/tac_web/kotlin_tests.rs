#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use std::path::{Path, PathBuf};

struct KotlinFixtures;

impl KotlinFixtures {
    const FILES: [(&str, &str); 7] = [
        ("client/models/CounterStarted.kt", "class CounterStarted\n"),
        (
            "client/models/CounterAdded.kt",
            "data class CounterAdded(val count: Long)\n",
        ),
        (
            "client/models/SignalSent.kt",
            "data class SignalSent(val size: Int)\n",
        ),
        (
            "client/models/SignalEcho.kt",
            "data class SignalEcho(val size: Int)\n",
        ),
        (
            "client/services/CounterService.kt",
            "import models.CounterAdded\nimport models.CounterStarted\n\n@Service\n@Lifetime({\"value\":\"prototype\"})\n@NeedsAll\nclass CounterService {\n    var count: Long = 0\n    var label: String = \"\"\n    val applicationEventPublisher: ApplicationEventPublisher\n\n    \
             @OnStart\n    fun start() {\n        label += \" clicks\"\n        applicationEventPublisher.publishEvent(CounterStarted())\n    }\n\n    \
             fun increment(): Long {\n        count += 1\n        applicationEventPublisher.publishEvent(CounterAdded(count))\n        return count\n    }\n\n    \
             fun doubled(): Long = count * 2\n\n    \
             fun add(by: Long, why: String): String {\n        count += by\n        return \"$why $count\"\n    }\n\n    \
             fun fail(why: String): Int = throw IllegalStateException(\"counter $why\")\n}\n",
        ),
        (
            "client/services/SignalService.kt",
            "import models.SignalEcho\nimport models.SignalSent\n\n@Service\nclass SignalService {\n    var large: List<Int> = emptyList()\n    var echoed: Int = 0\n    val applicationEventPublisher: ApplicationEventPublisher\n\n    \
             fun send(size: Int) {\n        applicationEventPublisher.publishEvent(SignalSent(size))\n    }\n\n    \
             @OnEvent({\"condition\":\"value.size > 5\"})\n    fun record(value: SignalSent): SignalEcho {\n        large = large + value.size\n        return SignalEcho(value.size * 10)\n    }\n\n    \
             @OnEvent\n    @Order(1)\n    fun echo(value: SignalEcho) {\n        echoed = value.size\n    }\n}\n",
        ),
        (
            "client/services/StampService.kt",
            "@Service\nclass StampService {\n    private val clock = Clock()\n    private val secureRandom = SecureRandom()\n\n    \
             fun now(): Double = clock.nowMs()\n\n    fun zone(): String = clock.timeZone()\n\n    fun id(): String = secureRandom.uuid()\n\n    \
             fun permission(): String = Notifications().permission()\n}\n",
        ),
    ];

    const SETTINGS_FILES: [(&str, &str); 3] = [
        (
            "client/services/SettingsService.kt",
            "@Service\nclass SettingsService {\n    var title: String = \"Catalog\"\n    var pageSize: Long = 20\n\n    init {\n        val publicCatalog = mapOf(\n            \"title\" to \"Catalog\",\n            \"page\" to mapOf(\"size\" to 20),\n            \"cache\" to mapOf(\"enabled\" to true),\n        )\n        val published = mapOf(\"tac\" to mapOf(\"catalog\" to publicCatalog))\n        val setup = \"CatalogSetup\"\n        val catalog = Behaviors.properties(setup, published)\n        val values = Behaviors.values(\"PublicValuesSetup\", published)\n        title = values[\"title\"] as String\n        pageSize = catalog[\"pageSize\"] as Long\n        Behaviors.create(setup, Behaviors.Options(\n            \"public-catalog\", Behaviors.Signal(), profiles = listOf(\"public\"), properties = published\n        )).close()\n    }\n}\n",
        ),
        (
            "client/setups/CatalogSetup.kt",
            "@Setup\n@Settings({\"prefix\":\"TAC_CATALOG_\"})\n@WhenProfile(\"public\")\n@WhenSetting({\"name\":\"TAC_CATALOG_CACHE_ENABLED\",\"havingValue\":\"true\",\"matchIfMissing\":false})\nclass CatalogSetup {\n    @Min(1)\n    @Max(100)\n    var pageSize: Int = 20\n    var cacheEnabled: Boolean = false\n    var title: String = \"Catalog\"\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n",
        ),
        (
            "client/setups/PublicValuesSetup.kt",
            "@Setup\nclass PublicValuesSetup {\n    @Setting(\"${tac.catalog.title:Catalog}\")\n    var title: String = \"\"\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n",
        ),
    ];

    const STORAGE_FILES: [(&str, &str); 3] = [
        (
            "client/setups/BrowserSetup.kt",
            "@Setup\nclass BrowserSetup {\n    companion object {\n        \
             fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n",
        ),
        (
            "client/entities/NoteEntity.kt",
            "@Entity({\"adapter\":\"indexeddb\"})\n@Table(\"notes\")\nclass NoteEntity {\n    @Id\n    var id: String = \"\"\n    var text: String = \"\"\n}\n",
        ),
        (
            "client/services/NotesService.kt",
            "@Service\n@CacheSettings({\"name\":\"notes\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":8})\n\
             class NotesService {\n    \
             lateinit var behaviors: Behaviors.Context\n    \
             private var reads: Long = 0\n\n    \
             @OnStart\n    fun open() {\n        \
             behaviors = Behaviors.create(\"BrowserSetup\", Behaviors.Options(\"notes\", Behaviors.Signal()))\n    }\n\n    \
             @Cached\n    fun find(key: String): Any? {\n        reads += 1\n        return mapOf(\"key\" to key, \"reads\" to reads)\n    }\n\n    \
             @ClearCache({\"method\":\"find\"})\n    fun forget(key: String): Any? {\n        return key\n    }\n\n    \
             fun save(id: String, text: String): Any? {\n        \
             return behaviors.repository(\"NoteEntity\").save(mapOf(\"id\" to id, \"text\" to text))\n    }\n\n    \
             @AllOrNothing({\"adapter\":\"indexeddb\"})\n    fun saveThenRefuse(id: String): Any? {\n        \
             save(id, \"draft\")\n        throw IllegalStateException(\"the note was refused\")\n    }\n\n    \
             fun list(): Any? {\n        return behaviors.repository(\"NoteEntity\").find(mapOf(\"size\" to 10))\n    }\n}\n",
        ),
    ];

    const STORAGE_DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         console.info = () => {}\n\
         globalThis.indexedDB = { databases: async () => [], open: () => ({}) }\n\
         ;(0, eval)(await readFile('./tac-storage.js', 'utf8'))\n\
         const { default: NotesService } = await import('./NotesService.js')\n\
         const notes = new NotesService({})\n\
         notes.__tachyonStarted()\n\
         const first = notes.find('a')\n\
         const second = notes.find('a')\n\
         notes.forget('a')\n\
         const third = notes.find('a')\n\
         notes.save('n1', 'hello')\n\
         let refused = ''\n\
         try { notes.saveThenRefuse('n2') } catch (error) { refused = error.message }\n\
         console.log(JSON.stringify({ first, second, third, refused, listed: notes.list() }))\n\
         process.exit(0)\n";

    const SETTINGS_DRIVER: &str = "const { default: SettingsService } = await import('./SettingsService.js')\n\
         const settings = new SettingsService({})\n\
         console.log(JSON.stringify({ title: settings.title, pageSize: settings.pageSize }))\n\
         process.exit(0)\n";

    const SERVICES: [(&str, &str); 3] = [
        ("counter", "CounterService"),
        ("signal", "SignalService"),
        ("stamp", "StampService"),
    ];

    const COUNTER_DRIVER: &str = "const { default: CounterService } = await import('./CounterService.js')\n\
         const events = []\n\
         const one = new CounterService({ label: 'many', ignored: 1 }), two = new CounterService({})\n\
         for (const [tag, owner] of [['one', one], ['two', two]]) {\n\
         \x20 Object.defineProperty(owner, '__tachyonPublisher', { value: { publishEvent: (event, value) => events.push([tag, event, value]) } })\n\
         \x20 owner.__tachyonStarted()\n\
         }\n\
         one.increment(); one.increment(); two.increment()\n\
         const doubled = one.doubled()\n\
         one.count = 40\n\
         const after = one.increment()\n\
         const added = one.add(new Event('click'), 9, 'now')\n\
         let failed = ''\n\
         try { one.fail('broke') } catch (error) { failed = error.message }\n\
         let bigStart = '', bigCall = '', largeStart = ''\n\
         try { new CounterService({ label: 'x'.repeat(1100000) }) } catch (error) { bigStart = error.message }\n\
         try { new CounterService({ label: 'x'.repeat(921600) }).__tachyonDispose() } catch (error) { largeStart = error.message }\n\
         try { one.add(new Event('click'), 1, 'x'.repeat(70000)) } catch (error) { bigCall = error.message }\n\
         one.__tachyonDispose()\n\
         let stale = false\n\
         try { one.count } catch { stale = true }\n\
         console.log(JSON.stringify({ doubled, after, other: two.count, label: two.label, added, failed, stale, events, keys: Object.keys(two), bigStart, bigCall, largeStart }))\n\
         process.exit(0)\n";

    const SIGNAL_DRIVER: &str = "const { default: SignalService, __tachyon_event_plan__: plan } = await import('./SignalService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const events = []\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         for (const event of ['SignalSent', 'SignalEcho']) bus.attach({ receive: (value) => events.push([event, value]) }, { listeners: [{ method: 'receive', event, order: -100 }] }, {})\n\
         const signal = new SignalService({})\n\
         bus.attach(signal, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(signal)\n\
         signal.send(3); signal.send(8)\n\
         console.log(JSON.stringify({ large: signal.large, echoed: signal.echoed, events }))\n\
         process.exit(0)\n";

    const STAMP_DRIVER: &str = "const answers = { 'clock.now': { value: 5 }, 'clock.timeZone': { value: 'UTC' }, 'secureRandom.uuid': { value: 'id-1' } }\n\
         globalThis[Symbol.for('tachyon.tac.web')] = {\n\
         \x20 host: (text) => { const asked = JSON.parse(text); return JSON.stringify(answers[`${asked.api}.${asked.call}`] ?? { error: 'Notifications are blocked.' }) },\n\
         \x20 calling: (owner, work) => work(),\n\
         }\n\
         const { default: StampService } = await import('./StampService.js')\n\
         const stamp = new StampService({})\n\
         let blocked = ''\n\
         try { stamp.permission() } catch (error) { blocked = error.message }\n\
         console.log(JSON.stringify({ now: stamp.now(), zone: stamp.zone(), id: stamp.id(), blocked }))\n\
         process.exit(0)\n";

    const OLD_BROWSER_DRIVER: &str = "WebAssembly.validate = () => false\n\
         let refused = ''\n\
         try { await import('./StampService.js') } catch (error) { refused = error.message }\n\
         console.log(JSON.stringify({ refused }))\n\
         process.exit(0)\n";
}

pub(super) fn kotlin_web_ready() -> bool {
    let library = std::env::var_os("TAC_KOTLIN_WASM_STDLIB").map(PathBuf::from);
    let has_library = library.is_some_and(|path| path.is_file());
    let ready = has_library
        && crate::program_probe::ProgramProbe::answers("kotlinc-js", "-version")
        && NodeDriver::available();
    assert!(
        ready || std::env::var_os("TAC_KOTLIN_WEB_REQUIRE_TOOLCHAINS").is_none(),
        "kotlinc-js, TAC_KOTLIN_WASM_STDLIB and node are required on this runner"
    );
    ready
}

fn write(root: &Path, relative: &str, text: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
    std::fs::write(&path, text).expect("file");
    path
}

fn run_driver(root: &Path, script: &str) -> serde_json::Value {
    std::fs::write(root.join("drive.mjs"), script).expect("driver");
    let output =
        NodeDriver::output(NodeDriver::command().arg("drive.mjs").current_dir(root)).expect("node");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_str(stdout.trim()).expect("report")
}

async fn publish_for_development(root: &Path, services: &[(String, String, WebLanguage, PathBuf)]) {
    let built = build_all(
        root,
        services,
        &BuildJobs::new(1),
        crate::BuildProfile::Development,
    )
    .await
    .expect("the Kotlin behavior runtime builds for the web");
    for (name, bytes) in built.files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, bytes).expect("publish");
    }
}

async fn build_kotlin_services(root: &Path) {
    for (relative, text) in KotlinFixtures::FILES {
        write(root, relative, text);
    }
    let services: Vec<_> = KotlinFixtures::SERVICES
        .iter()
        .map(|(instance, class)| {
            let source = root.join(format!("client/services/{class}.kt"));
            (
                String::from(*instance),
                String::from(*class),
                WebLanguage::Kotlin,
                source,
            )
        })
        .collect();
    let built = build_all(
        root,
        &services,
        &BuildJobs::new(1),
        crate::BuildProfile::Release,
    )
    .await
    .expect("the Kotlin services build");
    for (name, bytes) in built.files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, bytes).expect("publish");
    }
    std::fs::write(
        root.join("application-events.mjs"),
        concat!(
            include_str!("../template/application-events.js"),
            "\nexport default TacApplicationEvents\n"
        ),
    )
    .expect("event runtime");
}

#[tokio::test]
async fn kotlin_services_run_as_one_webassembly_program() {
    if !kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, TAC_KOTLIN_WASM_STDLIB or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    build_kotlin_services(root.path()).await;
    assert!(root.path().join("kotlin/services.wasm").is_file());
    assert!(root.path().join("tachyon-kotlin.js").is_file());

    let counter = run_driver(root.path(), KotlinFixtures::COUNTER_DRIVER);
    assert_eq!(counter["doubled"], 4);
    assert_eq!(counter["after"], 41);
    assert_eq!(counter["other"], 1);
    assert_eq!(counter["label"], " clicks");
    assert_eq!(counter["added"], "now 50");
    assert_eq!(counter["failed"], "Web service counter: counter broke");
    assert_eq!(
        counter["bigStart"],
        "Web service counter: The request is 1100115 bytes. The limit is 1048576 bytes. Pass less data to the service."
    );
    assert_eq!(
        counter["bigCall"],
        "Web service counter: The request to add is 70121 bytes. The limit is 65536 bytes. Pass less data to the service."
    );
    assert_eq!(counter["largeStart"], "");
    assert_eq!(counter["stale"], true);
    assert_eq!(counter["keys"], serde_json::json!(["count", "label"]));
    assert_eq!(
        counter["events"],
        serde_json::json!([
            ["one", "CounterStarted", {}],
            ["two", "CounterStarted", {}],
            ["one", "CounterAdded", {"count": 1}],
            ["one", "CounterAdded", {"count": 2}],
            ["two", "CounterAdded", {"count": 1}],
            ["one", "CounterAdded", {"count": 41}],
        ])
    );

    let signal = run_driver(root.path(), KotlinFixtures::SIGNAL_DRIVER);
    assert_eq!(signal["large"], serde_json::json!([8]));
    assert_eq!(signal["echoed"], 80);
    assert_eq!(
        signal["events"],
        serde_json::json!([
            ["SignalSent", {"size": 3}],
            ["SignalSent", {"size": 8}],
            ["SignalEcho", {"size": 80}],
        ])
    );

    let stamp = run_driver(root.path(), KotlinFixtures::STAMP_DRIVER);
    assert_eq!(
        stamp,
        serde_json::json!({"now": 5, "zone": "UTC", "id": "id-1", "blocked": "Web service stamp: Notifications are blocked."})
    );

    let old = run_driver(root.path(), KotlinFixtures::OLD_BROWSER_DRIVER);
    assert!(
        old["refused"]
            .as_str()
            .is_some_and(|message| message.contains("WebAssembly garbage collection")),
        "{old}"
    );
}

#[tokio::test]
async fn a_kotlin_service_uses_the_behavior_runtime_as_webassembly() {
    if !kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, TAC_KOTLIN_WASM_STDLIB or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in KotlinFixtures::SETTINGS_FILES {
        write(root.path(), relative, text);
    }
    let source = root.path().join("client/services/SettingsService.kt");
    let services = [(
        String::from("settings"),
        String::from("SettingsService"),
        WebLanguage::Kotlin,
        source,
    )];
    publish_for_development(root.path(), &services).await;
    let report = run_driver(root.path(), KotlinFixtures::SETTINGS_DRIVER);
    assert_eq!(
        report,
        serde_json::json!({"title": "Catalog", "pageSize": 20})
    );
}

#[tokio::test]
async fn a_kotlin_service_reads_the_builtin_cache_and_store_synchronously() {
    if !kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, TAC_KOTLIN_WASM_STDLIB or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in KotlinFixtures::STORAGE_FILES {
        write(root.path(), relative, text);
    }
    write(
        root.path(),
        "tac-storage.js",
        include_str!("../template/tac-storage.js"),
    );
    let services = [(
        String::from("notes"),
        String::from("NotesService"),
        WebLanguage::Kotlin,
        root.path().join("client/services/NotesService.kt"),
    )];
    publish_for_development(root.path(), &services).await;
    let report = run_driver(root.path(), KotlinFixtures::STORAGE_DRIVER);
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
