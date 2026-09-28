#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use std::path::{Path, PathBuf};
use std::process::Command;

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
         one.__tachyonDispose()\n\
         let stale = false\n\
         try { one.count } catch { stale = true }\n\
         console.log(JSON.stringify({ doubled, after, other: two.count, label: two.label, added, failed, stale, events, keys: Object.keys(two) }))\n\
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

fn kotlin_web_ready() -> bool {
    let library = std::env::var_os("TAC_KOTLIN_WASM_STDLIB").map(PathBuf::from);
    let has_library = library.is_some_and(|path| path.is_file());
    let runs = |program: &str, flag: &str| {
        Command::new(program)
            .arg(flag)
            .output()
            .is_ok_and(|output| output.status.success())
    };
    has_library && runs("kotlinc-js", "-version") && runs("node", "--version")
}

fn write(root: &Path, relative: &str, text: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
    std::fs::write(&path, text).expect("file");
    path
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
