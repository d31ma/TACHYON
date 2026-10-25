#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use std::path::{Path, PathBuf};
use std::process::Command;

struct JavaFixtures;

impl JavaFixtures {
    const COUNTER_SERVICE: &str = "import java.util.List;\n\n@Service\n@Lifetime({\"value\":\"prototype\"})\nclass CounterService {\n    int count = 0;\n    String label = \"\";\n    double price = 1.5;\n    final String runtime = \"Java\";\n    private int secret = 7;\n\n    \
         @OnStart\n    void start() {\n        label += \" clicks\";\n    }\n\n    \
         int increment() {\n        count += 1;\n        return count;\n    }\n\n    \
         int doubled() {\n        return count * 2;\n    }\n\n    \
         String add(int by, String why) {\n        count += by;\n        return why + \" \" + count;\n    }\n\n    \
         double scale(double factor) {\n        price *= factor;\n        return price;\n    }\n\n    \
         long total(List<Long> values) {\n        long sum = secret - 7;\n        for (long value : values) {\n            sum += value;\n        }\n        return sum;\n    }\n\n    \
         int fail(String why) throws Exception {\n        throw new Exception(\"counter \" + why);\n    }\n}\n";

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
         const total = one.total([1, 2, 2147483648])\n\
         let failed = '', readOnly = ''\n\
         try { one.fail('broke') } catch (error) { failed = error.message }\n\
         try { one.runtime = 'Kotlin' } catch (error) { readOnly = error.message }\n\
         let bigStart = '', bigCall = '', largeStart = ''\n\
         try { new CounterService({ label: 'x'.repeat(1100000) }) } catch (error) { bigStart = error.message }\n\
         try { new CounterService({ label: 'x'.repeat(921600) }).__tachyonDispose() } catch (error) { largeStart = error.message }\n\
         try { one.add(new Event('click'), 1, 'x'.repeat(70000)) } catch (error) { bigCall = error.message }\n\
         one.__tachyonDispose()\n\
         let stale = false\n\
         try { one.count } catch { stale = true }\n\
         console.log(JSON.stringify({ doubled, after, other: two.count, label: two.label, added, scaled, total, runtime: two.runtime, failed, readOnly, stale, keys: Object.keys(two).sort(), bigStart, bigCall, largeStart }))\n\
         process.exit(0)\n";
    const OLD_BROWSER_DRIVER: &str = "const validate = WebAssembly.validate\n\
         WebAssembly.validate = (bytes) => !bytes.includes(31) && validate(bytes)\n\
         let refused = ''\n\
         try { await import('./CounterService.js') } catch (error) { refused = error.message }\n\
         console.log(JSON.stringify({ refused }))\n\
         process.exit(0)\n";
    const LIBRARY_FILES: [(&str, &str); 7] = [
        (
            "client/models/Author.java",
            "class Author {\n    String name = \"\";\n}\n",
        ),
        (
            "client/models/Size.java",
            "enum Size {\n    SHORT, LONG\n}\n",
        ),
        (
            "client/models/Guide.java",
            "import java.util.List;\n\nrecord Guide(String title, int pages, double rating, boolean draft, List<String> tags, String note, Size size, Author author) {}\n",
        ),
        (
            "client/models/GuideSaved.java",
            "record GuideSaved(String title, int pages) {}\n",
        ),
        (
            "client/models/GuideEcho.java",
            "record GuideEcho(int pages) {}\n",
        ),
        (
            "client/models/ShelfChanged.java",
            "@Topic\nrecord ShelfChanged(int count) {}\n",
        ),
        (
            "client/services/LibraryService.java",
            "import java.util.ArrayList;\nimport java.util.List;\nimport models.Guide;\nimport models.GuideEcho;\nimport models.GuideSaved;\nimport models.ShelfChanged;\n\n@Service\nclass LibraryService {\n    Guide latest = null;\n    List<Integer> heard = new ArrayList<>();\n    ApplicationEventPublisher applicationEventPublisher;\n\n    \
             Guide longer(Guide guide, int extra) {\n        Guide copy = new Guide(guide.title(), guide.pages() + extra, guide.rating(), guide.draft(), guide.tags(), guide.note(), guide.size(), guide.author());\n        latest = copy;\n        \
             applicationEventPublisher.publishEvent(new GuideSaved(copy.title(), copy.pages()));\n        return copy;\n    }\n\n    \
             List<Guide> shelf() {\n        return latest == null ? List.of() : List.of(latest);\n    }\n\n    \
             @OnEvent({\"condition\":\"value.pages > 10\"})\n    GuideEcho saved(GuideSaved value) {\n        heard.add(value.pages());\n        return new GuideEcho(value.pages() * 10);\n    }\n\n    \
             @OnEvent\n    void shelfChanged(ShelfChanged value) {\n        heard.add(value.count());\n    }\n}\n",
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
         const guide = { title: 'Java', pages: 8, rating: 4.5, draft: false, tags: ['wasm'], note: null, size: 'LONG', author: { name: 'Ann' } }\n\
         const small = library.longer(guide, 1)\n\
         const large = library.longer(guide, 4)\n\
         for (const source of sources) source.onmessage({ data: JSON.stringify({ count: 3 }), lastEventId: '1' })\n\
         let refused = ''\n\
         try { library.longer({ title: 'broken' }, 1) } catch (error) { refused = error.message }\n\
         console.log(JSON.stringify({ small, large, latest: library.latest, shelf: library.shelf(), heard: library.heard, events, topics: sources.map((source) => source.url), refused }))\n\
         process.exit(0)\n";

    const STORAGE_FILES: [(&str, &str); 3] = [
        (
            "client/setups/BrowserSetup.java",
            "@Setup\nclass BrowserSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return new Behaviors.Configuration();\n    }\n}\n",
        ),
        (
            "client/entities/NoteEntity.java",
            "@Entity({\"adapter\":\"indexeddb\"})\n@Table(\"notes\")\nclass NoteEntity {\n    @Id\n    String id = \"\";\n    String text = \"\";\n}\n",
        ),
        (
            "client/services/NotesService.java",
            "import java.util.Map;\n\n@Service\n@CacheSettings({\"name\":\"notes\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":8})\nclass NotesService {\n    \
             Behaviors.Context behaviors;\n    private long reads = 0;\n\n    \
             @OnStart\n    void open() {\n        behaviors = Behaviors.create(\"BrowserSetup\", new Behaviors.Options(\"notes\", new Behaviors.Signal()));\n    }\n\n    \
             @Cached\n    Object find(String key) {\n        reads += 1;\n        return Map.of(\"key\", key, \"reads\", reads);\n    }\n\n    \
             @ClearCache({\"method\":\"find\"})\n    Object forget(String key) {\n        return key;\n    }\n\n    \
             Object save(String id, String text) {\n        return notes().save(Map.of(\"id\", id, \"text\", text));\n    }\n\n    \
             @AllOrNothing({\"adapter\":\"indexeddb\"})\n    Object saveThenRefuse(String id) {\n        save(id, \"draft\");\n        throw new IllegalStateException(\"the note was refused\");\n    }\n\n    \
             Object list() {\n        return notes().find(Map.<String, Object>of(\"size\", 10));\n    }\n\n    \
             private Behaviors.Repository notes() {\n        return behaviors.repository(\"NoteEntity\");\n    }\n}\n",
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
         const first = notes.find('a')\n\
         const second = notes.find('a')\n\
         notes.forget('a')\n\
         const third = notes.find('a')\n\
         notes.save('n1', 'hello')\n\
         let refused = ''\n\
         try { notes.saveThenRefuse('n2') } catch (error) { refused = error.message }\n\
         console.log(JSON.stringify({ first, second, third, refused, listed: notes.list() }))\n\
         process.exit(0)\n";
}

pub(super) fn java_web_ready() -> bool {
    let runs = |program: &str, flag: &str| {
        Command::new(program)
            .arg(flag)
            .output()
            .is_ok_and(|output| output.status.success())
    };
    crate::native::JavaWebServices::teavm_installed()
        && runs("javac", "-version")
        && runs("node", "--version")
}

pub(super) fn run_driver(root: &Path, script: &str) -> serde_json::Value {
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

async fn build_counter(root: &Path, profile: crate::BuildProfile) -> Vec<String> {
    let source = root.join("client/services/CounterService.java");
    std::fs::create_dir_all(source.parent().expect("parent")).expect("folder");
    std::fs::write(&source, JavaFixtures::COUNTER_SERVICE).expect("service");
    let services = [(
        String::from("counter"),
        String::from("CounterService"),
        WebLanguage::Java,
        source,
    )];
    let built = build_all(root, &services, &BuildJobs::new(1), profile)
        .await
        .expect("the Java service builds");
    for (name, bytes) in built.files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, bytes).expect("publish");
    }
    built.notes
}

#[tokio::test]
async fn a_java_service_runs_in_the_page_as_webassembly() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let notes = build_counter(root.path(), crate::BuildProfile::Development).await;
    assert!(notes.is_empty(), "{notes:?}");
    assert!(root.path().join("java/services.wasm").is_file());
    let runtime = std::fs::read_to_string(root.path().join("java/runtime.js")).expect("runtime");
    assert!(
        !runtime.contains("new Function"),
        "runtime.js makes code from text"
    );
    assert!(root.path().join("tachyon-java.js").is_file());

    let counter = run_driver(root.path(), JavaFixtures::COUNTER_DRIVER);
    assert_eq!(
        counter,
        serde_json::json!({
            "doubled": 4,
            "after": 41,
            "other": 1,
            "label": " clicks",
            "added": "now 50",
            "scaled": 2.5,
            "total": 2_147_483_651_i64,
            "runtime": "Java",
            "failed": "Web service counter: counter broke",
            "readOnly": "Web service counter: Companion field is read-only: runtime",
            "stale": true,
            "keys": ["count", "label", "price", "runtime"],
            "bigStart": "Web service counter: The request is 1100115 bytes. The limit is 1048576 bytes. Pass less data to the service.",
            "bigCall": "Web service counter: The request to add is 70121 bytes. The limit is 65536 bytes. Pass less data to the service.",
            "largeStart": "",
        })
    );

    let old = run_driver(root.path(), JavaFixtures::OLD_BROWSER_DRIVER);
    assert_eq!(
        old["refused"],
        "Java services need a browser with WebAssembly garbage collection and exception references (exnref): Chrome or Edge 137, Firefox 131 or Safari 18.4 and newer.",
        "a browser with WasmGC but without try_table refuses before it loads the module"
    );
}

#[tokio::test]
async fn a_java_release_build_is_made_smaller_by_binaryen() {
    if !java_web_ready() || !crate::wasm_optimizer::WasmOptimizer::is_installed() {
        eprintln!("skipping: TeaVM, javac, node or wasm-opt is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let development = build_counter(root.path(), crate::BuildProfile::Development).await;
    assert!(development.is_empty(), "{development:?}");
    let unoptimized = std::fs::metadata(root.path().join("java/services.wasm"))
        .expect("development binary")
        .len();
    let notes = build_counter(root.path(), crate::BuildProfile::Release).await;
    assert_eq!(
        notes,
        ["Java: TeaVM WebAssembly release build — optimized by Binaryen"]
    );
    let optimized = std::fs::metadata(root.path().join("java/services.wasm"))
        .expect("release binary")
        .len();
    assert!(optimized < unoptimized, "{optimized} < {unoptimized}");
    let counter = run_driver(root.path(), JavaFixtures::COUNTER_DRIVER);
    assert_eq!(counter["after"], 41);
}

#[tokio::test]
async fn a_java_service_names_the_annotation_it_cannot_use() {
    let root = tempfile::tempdir().expect("project");
    let source = root.path().join("client/services/NoteService.java");
    std::fs::create_dir_all(source.parent().expect("parent")).expect("folder");
    std::fs::write(
        &source,
        "@Service\nclass NoteService {\n    String text = \"\";\n\n    @SafeVarargs\n    final String read(String... parts) {\n        return text;\n    }\n}\n",
    )
    .expect("service");
    let services = [(
        String::from("note"),
        String::from("NoteService"),
        WebLanguage::Java,
        source,
    )];
    let Err(failure) = build_all(
        root.path(),
        &services,
        &BuildJobs::new(1),
        crate::BuildProfile::Development,
    )
    .await
    else {
        panic!("an unsupported annotation is refused");
    };
    let text = failure.to_string();
    assert!(text.contains("TY1405"), "{text}");
    assert!(
        text.contains("@SafeVarargs does not work in a Java web service."),
        "{text}"
    );
}

pub(super) fn write(root: &Path, relative: &str, text: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
    std::fs::write(&path, text).expect("file");
    path
}

pub(super) async fn publish(root: &Path, instance: &str, class: &str) {
    let services = [(
        String::from(instance),
        String::from(class),
        WebLanguage::Java,
        root.join(format!("client/services/{class}.java")),
    )];
    let built = build_all(
        root,
        &services,
        &BuildJobs::new(1),
        crate::BuildProfile::Development,
    )
    .await
    .expect("the Java service builds");
    for (name, bytes) in built.files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, bytes).expect("publish");
    }
}

#[tokio::test]
async fn a_java_service_exchanges_models_and_events_with_the_page() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in JavaFixtures::LIBRARY_FILES {
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
    let report = run_driver(root.path(), JavaFixtures::LIBRARY_DRIVER);
    let guide = |pages: i64| serde_json::json!({"title": "Java", "pages": pages, "rating": 4.5, "draft": false, "tags": ["wasm"], "note": null, "size": "LONG", "author": {"name": "Ann"}});
    assert_eq!(report["small"], guide(9));
    assert_eq!(report["large"], guide(12));
    assert_eq!(report["latest"], guide(12));
    assert_eq!(report["shelf"], serde_json::json!([guide(12)]));
    assert_eq!(report["heard"], serde_json::json!([12, 3]));
    assert_eq!(
        report["events"],
        serde_json::json!([
            ["GuideSaved", {"title": "Java", "pages": 9}],
            ["GuideSaved", {"title": "Java", "pages": 12}],
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
async fn a_java_service_reads_the_builtin_cache_and_store_synchronously() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in JavaFixtures::STORAGE_FILES {
        write(root.path(), relative, text);
    }
    write(
        root.path(),
        "tac-storage.js",
        include_str!("../template/tac-storage.js"),
    );
    publish(root.path(), "notes", "NotesService").await;
    let report = run_driver(root.path(), JavaFixtures::STORAGE_DRIVER);
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
