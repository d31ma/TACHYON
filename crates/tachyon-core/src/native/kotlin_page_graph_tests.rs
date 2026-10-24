#![allow(clippy::expect_used)]

use super::managed_capability_tests::ManagedCapabilityCheck;
use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;

struct KotlinPageGraphCheck;

impl KotlinPageGraphCheck {
    const SERVICES: [(&'static str, &'static str, &'static str); 5] = [
        (
            "counter",
            "CounterService",
            "@Service\nclass CounterService {\n    @Needs\n    val picker: FilePicker\n    var count: Int = 0\n    fun increment(): Int { count += 1; return count }\n    fun read(): Int = count\n    fun choose(): String = picker.open().status.wire\n    @OnStart\n    fun started() { Probe.events.add(\"counter.start\") }\n    @OnStop\n    fun stopped() { Probe.events.add(\"counter.stop:\" + picker.open().status.wire) }\n}\n",
        ),
        (
            "tally",
            "TallyService",
            "@Service\n@Lifetime({\"value\":\"prototype\"})\nclass TallyService {\n    var count: Int = 0\n    fun increment(): Int { count += 1; return count }\n    fun read(): Int = count\n    @OnStart\n    fun started() { Probe.events.add(\"tally.start\") }\n    @OnStop\n    fun stopped() { Probe.events.add(\"tally.stop\") }\n}\n",
        ),
        (
            "fragile",
            "FragileService",
            "@Service\nclass FragileService {\n    var ready: Boolean = false\n    @OnStart\n    fun started() {\n        Probe.events.add(\"fragile.start\")\n        check(!Probe.fail) { \"private startup detail\" }\n        ready = true\n    }\n    @OnStop\n    fun stopped() { Probe.events.add(\"fragile.stop\") }\n}\n",
        ),
        (
            "label",
            "LabelService",
            "@Service\n@NeedsAll\n@Lifetime({\"value\":\"singleton\"})\nclass LabelService {\n    lateinit var label: String\n    var observed: String = \"\"\n    @OnStart\n    fun started() { observed = label }\n}\n",
        ),
        (
            "constructor",
            "ConstructorService",
            "@Service\nclass ConstructorService {\n    var ready: Boolean = Probe.construct()\n}\n",
        ),
    ];

    const SUPPORT: &'static str = r#"
object Probe {
    const val CAPABILITY_LIMIT = 256
    val events = mutableListOf<String>()
    val requests = mutableListOf<String>()
    var fail = false
    var failConstruction = false
    fun construct(): Boolean {
        check(!failConstruction) { "private constructor detail" }
        return true
    }
}

fun ask(route: String, service: String, owner: String, op: String, extra: String = ""): String =
    tacInvokeOwned("{\"route\":\"$route\",\"service\":\"$service\",\"owner\":\"$owner\",\"op\":\"$op\"$extra}")

fun call(service: String, owner: String, name: String, route: String = "/"): String =
    ask(route, service, owner, "call", ",\"name\":\"$name\",\"args\":[]")

fun field(service: String, owner: String, name: String): String =
    ask("/", service, owner, "get", ",\"name\":\"$name\"")

fun started(route: String, service: String, owner: String, extra: String = "") {
    val answer = ask(route, service, owner, "init", extra)
    check(!answer.contains("error")) { "init $service for $owner failed: $answer" }
}

fun expect(actual: Any?, expected: Any?, message: String) {
    check(actual == expected) { "$message: expected $expected, got $actual" }
}

fun retainedRoutes(): Int {
    val field = TacPageServices::class.java.getDeclaredField("pages")
    field.isAccessible = true
    return (field.get(TacPageServices) as Map<*, *>).size
}

fun retainedCapabilities(): Int {
    val field = NativeCapabilities::class.java.getDeclaredField("owners")
    field.isAccessible = true
    return (field.get(NativeCapabilities) as Map<*, *>).size
}

fun clearPage() {
    expect(tacInvokeOwned("{\"op\":\"host-dispose\"}"), "{\"value\":null}", "page disposal succeeds")
    expect(TacPageServices.size, 0, "page disposal releases every node")
    expect(retainedRoutes(), 0, "page disposal releases every route")
    expect(retainedCapabilities(), 0, "page disposal releases every capability owner")
}

"#;

    const CALLER: &'static str = r#"
fun main() {
    val owners = (1..7).map { "00000000-0000-4000-8000-00000000000$it" }
    val (one, two, three, four, five) = owners
    val six = owners[5]
    val seven = owners[6]
    NativeCapabilities.call = { capability, request ->
        Probe.requests.add("$capability $request")
        if (capability == "filePicker.open") "{\"status\":\"cancelled\"}" else "{\"status\":\"unsupported\"}"
    }

    started("/", "counter", one)
    started("/", "counter", two)
    expect(Probe.events, listOf("counter.start"), "a page singleton starts once")
    expect(call("counter", one, "increment"), "{\"value\":1}", "the first owner changes the singleton")
    expect(call("counter", two, "read"), "{\"value\":1}", "two owners share the page singleton")
    expect(call("counter", one, "choose"), "{\"value\":\"cancelled\"}", "the singleton reaches its capability")
    val opened = Probe.requests.last { it.startsWith("filePicker.open") }
    check(opened.contains("00000000-0000-4000-a000-") && !opened.contains(one)) { "the singleton capability belongs to the page node: $opened" }
    ask("/", "counter", one, "dispose")
    expect(call("counter", two, "choose"), "{\"value\":\"cancelled\"}", "disposing the first owner keeps the shared capability")
    expect(call("counter", two, "read"), "{\"value\":1}", "disposing the first owner keeps the shared state")
    ask("/", "counter", two, "dispose")
    expect(Probe.events, listOf("counter.start"), "the page singleton survives zero owners")
    started("/", "counter", two)
    expect(call("counter", two, "read"), "{\"value\":1}", "a new owner on the page reuses the singleton")
    started("/other", "counter", three)
    expect(call("counter", three, "read", "/other"), "{\"value\":0}", "another route has its own singleton")
    expect(Probe.events, listOf("counter.start", "counter.start"), "each route starts its singleton once")

    started("/", "tally", one)
    started("/", "tally", four)
    expect(Probe.events.count { it == "tally.start" }, 2, "each prototype owner starts its own instance")
    expect(call("tally", one, "increment"), "{\"value\":1}", "the prototype owner changes its instance")
    expect(call("tally", four, "read"), "{\"value\":0}", "another prototype owner is isolated")
    ask("/", "tally", one, "dispose")
    expect(Probe.events.count { it == "tally.stop" }, 1, "a prototype stops with its owner")
    check(ask("/", "tally", one, "dispose").contains("error")) { "a disposed owner is stale" }
    expect(Probe.events.count { it == "tally.stop" }, 1, "a prototype stops exactly once")

    Probe.fail = true
    check(ask("/", "fragile", five, "init").contains("error")) { "a failed start refuses the owner" }
    expect(Probe.events.takeLast(2), listOf("fragile.start", "fragile.stop"), "a failed start stops the fresh singleton")
    expect(call("counter", two, "read"), "{\"value\":1}", "a failed owner keeps the existing page singletons")
    Probe.fail = false
    started("/", "fragile", five)
    expect(Probe.events.last(), "fragile.start", "a failed singleton is constructed afresh")

    started("/", "label", six, ",\"props\":{\"label\":\"given\"}")
    expect(field("label", six, "observed"), "{\"value\":\"given\"}", "properties are applied before the singleton starts")
    started("/", "label", seven, ",\"props\":{\"label\":\"second\"}")
    expect(field("label", seven, "observed"), "{\"value\":\"given\"}", "a page singleton starts only once")
    ask("/", "label", six, "set", ",\"name\":\"label\",\"value\":\"changed\"")
    expect(field("label", seven, "label"), "{\"value\":\"changed\"}", "both owners write the same singleton")

    val before = Probe.events.size
    tacInvokeOwned("{\"op\":\"host-dispose\"}")
    expect(
        Probe.events.drop(before),
        listOf("fragile.stop", "tally.stop", "counter.stop:cancelled", "counter.stop:cancelled"),
        "page exit stops each page's services in reverse construction order while their capabilities still answer",
    )
    tacInvokeOwned("{\"op\":\"host-dispose\"}")
    expect(Probe.events.size, before + 4, "page exit stops each service exactly once")
    started("/", "counter", one)
    expect(call("counter", one, "read"), "{\"value\":0}", "page exit releases the singleton")
    expect(Probe.events.last(), "counter.start", "the next page starts a new singleton")
    check(ask("/", "counter", "00000000-0000-4000-a000-000000000001", "init").contains("error")) { "a reserved provider identity is not an owner" }
    println("PASS kotlin page graph")
}
"#;

    const CAPACITY_CALLER: &'static str = r#"
fun failedRoutes(owner: String) {
    Probe.failConstruction = true
    for (index in 0..TacPageServices.NODE_LIMIT) {
        check(ask("/unknown/$index", "missing", owner, "init").contains("TY_NATIVE_SERVICE")) { "an unknown service fails" }
        check(ask("/constructor/$index", "constructor", owner, "init").contains("TY_NATIVE_REQUIRED")) { "a failed constructor refuses the owner" }
    }
    Probe.failConstruction = false
    expect(TacPageServices.size, 0, "failed construction retains no nodes")
    expect(retainedRoutes(), 0, "failed construction retains no routes")
    expect(retainedCapabilities(), 0, "failed construction retains no capability owners")
    expect(TacNativeState.activations.size, 0, "failed construction retains no component owners")
}

fun rejectedCapability(route: String, service: String, owner: String) {
    val answer = ask(route, service, owner, "init")
    println("Kotlin real capability capacity rejection for $service: $answer")
    check(answer.contains("TY_NATIVE_REQUIRED")) { "the existing generic constructor error reports capability capacity: $answer" }
}

fun retainedCapacity(owner: String, second: String) {
    for (index in 0 until Probe.CAPABILITY_LIMIT) {
        started("/capacity/$index", "counter", owner)
    }
    expect(TacPageServices.size, Probe.CAPABILITY_LIMIT, "real factory singletons retain 256 nodes")
    expect(retainedRoutes(), Probe.CAPABILITY_LIMIT, "real factory singletons retain 256 routes")
    expect(retainedCapabilities(), Probe.CAPABILITY_LIMIT, "real factory reaches its 256 capability owners")
    expect(TacNativeState.activations.size, 1, "route switches use one component owner")
    started("/capacity/0", "counter", second)
    expect(call("counter", second, "increment", "/capacity/0"), "{\"value\":1}", "another owner reuses a singleton at real capability capacity")
    expect(call("counter", second, "choose", "/capacity/0"), "{\"value\":\"cancelled\"}", "the reused singleton keeps its real capability")
    expect(retainedCapabilities(), Probe.CAPABILITY_LIMIT, "singleton reuse allocates no capability owner")
    rejectedCapability("/over-capacity", "counter", owner)
    rejectedCapability("/capacity/0", "tally", owner)
    expect(TacPageServices.size, Probe.CAPABILITY_LIMIT, "refused allocations retain no new node")
    expect(retainedRoutes(), Probe.CAPABILITY_LIMIT, "refused allocations retain no empty route")
    expect(retainedCapabilities(), Probe.CAPABILITY_LIMIT, "refused allocations retain no capability owner")
    clearPage()
    started("/after-exit", "counter", owner)
    expect(TacPageServices.size, 1, "page exit restores singleton capacity")
    expect(retainedCapabilities(), 1, "page exit restores capability capacity")
    clearPage()
}

fun recoveredCapacity(owner: String, second: String) {
    for (index in 0 until Probe.CAPABILITY_LIMIT - 1) {
        started("/recovery/$index", "counter", owner)
    }
    Probe.fail = true
    check(ask("/failed-start", "fragile", second, "init").contains("error")) { "a failed start refuses the owner" }
    Probe.fail = false
    expect(TacPageServices.size, Probe.CAPABILITY_LIMIT - 1, "failed startup releases its node")
    expect(retainedRoutes(), Probe.CAPABILITY_LIMIT - 1, "failed startup releases its empty route")
    expect(retainedCapabilities(), Probe.CAPABILITY_LIMIT - 1, "failed startup releases its capability owner")
    started("/prototype", "tally", second)
    expect(TacPageServices.size, Probe.CAPABILITY_LIMIT, "a prototype uses the recovered node slot")
    expect(retainedCapabilities(), Probe.CAPABILITY_LIMIT, "a prototype uses the recovered capability slot")
    expect(ask("/prototype", "tally", second, "dispose"), "{\"value\":null}", "prototype owner disposal succeeds")
    expect(TacPageServices.size, Probe.CAPABILITY_LIMIT - 1, "prototype disposal releases its node")
    expect(retainedRoutes(), Probe.CAPABILITY_LIMIT - 1, "prototype disposal releases its empty route")
    expect(retainedCapabilities(), Probe.CAPABILITY_LIMIT - 1, "prototype disposal releases its capability owner")
    started("/failed-start", "fragile", second)
    expect(TacPageServices.size, Probe.CAPABILITY_LIMIT, "retry after failed startup uses the recovered node slot")
    expect(retainedCapabilities(), Probe.CAPABILITY_LIMIT, "retry after failed startup uses the recovered capability slot")
    clearPage()
}

fun main() {
    val owner = "00000000-0000-4000-8000-000000000001"
    val second = "00000000-0000-4000-8000-000000000002"
    NativeCapabilities.call = { _, _ -> "{\"status\":\"cancelled\"}" }
    failedRoutes(owner)
    retainedCapacity(owner, second)
    recoveredCapacity(owner, second)
    println("PASS kotlin real capability capacity 256")
}
"#;

    const GUARD_CALLER: &'static str = r#"
fun main() {
    val owner = "00000000-0000-4000-8000-000000000001"
    val second = "00000000-0000-4000-8000-000000000002"
    NativeCapabilities.call = { _, _ -> "{\"status\":\"cancelled\"}" }
    started("/guard", "counter", owner)
    val field = TacPageServices::class.java.getDeclaredField("pages")
    field.isAccessible = true
    @Suppress("UNCHECKED_CAST")
    val pages = field.get(TacPageServices) as MutableMap<String, LinkedHashMap<Pair<String, String>, TacServiceNode>>
    val saved = LinkedHashMap(pages)
    val node = pages.getValue("/guard").getValue("counter" to "")
    try {
        for (index in 1 until TacPageServices.NODE_LIMIT) {
            pages["/synthetic-unit/$index"] = linkedMapOf(("counter" to "") to node)
        }
        expect(TacPageServices.size, TacPageServices.NODE_LIMIT, "synthetic aliases fill the 4096-entry page guard")
        expect(retainedCapabilities(), 1, "synthetic aliases leave the real capability registry intact")
        expect(TacPageServices.admits("/guard", "counter", second), true, "the unit guard permits an existing key")
        expect(TacPageServices.admits("/new-unit-route", "counter", second), false, "the unit guard refuses a new singleton key")
        expect(TacPageServices.admits("/guard", "tally", second), false, "the unit guard refuses a new prototype key")
        started("/guard", "counter", second)
        expect(retainedCapabilities(), 1, "init reuses the real singleton without allocating at the synthetic entry limit")
        check(ask("/new-unit-route", "counter", owner, "init").contains("Too many native services.")) { "init refuses a new singleton before its capability factory" }
        check(ask("/guard", "tally", owner, "init").contains("Too many native services.")) { "init refuses a new prototype before its capability factory" }
        expect(retainedCapabilities(), 1, "init rejection leaves the real capability registry intact")
    } finally {
        pages.clear()
        pages.putAll(saved)
        clearPage()
    }
    println("PASS kotlin synthetic page-entry guard unit 4096")
}
"#;

    fn run(caller: &str) -> String {
        let root = tempfile::tempdir().expect("project");
        let services = root.path().join("client/services");
        std::fs::create_dir_all(&services).expect("services");
        let inputs = Self::SERVICES
            .into_iter()
            .map(|(service, class, source)| {
                let path = services.join(format!("{class}.kt"));
                std::fs::write(&path, source).expect("service source");
                NativeCompanionInput {
                    snapshot_root: root.path().to_path_buf(),
                    language: NativeCompanion::Kotlin,
                    source: path,
                    service: service.to_owned(),
                    class: class.to_owned(),
                    web_assembly: false,
                    application_provider: false,
                }
            })
            .collect::<Vec<_>>();
        let registry = super::registry::source(&inputs, NativeCompanion::Kotlin)
            .expect("registry")
            .expect("source");
        ManagedCapabilityCheck::kotlin_jvm(
            root.path(),
            &registry,
            &format!("{}{}", Self::SUPPORT, caller),
            true,
        )
    }
}

#[test]
fn kotlin_native_services_are_page_singletons_unless_prototype() {
    if !ManagedCapabilityCheck::available("kotlinc", "-version")
        || !ManagedCapabilityCheck::available("java", "-version")
    {
        eprintln!("skipping: Kotlin JVM tools are unavailable");
        return;
    }
    let output = KotlinPageGraphCheck::run(KotlinPageGraphCheck::CALLER);
    println!("{output}");
    assert!(output.contains("PASS kotlin page graph"));
}

#[test]
fn kotlin_native_page_services_respect_capability_capacity_and_recover_after_failures() {
    if !ManagedCapabilityCheck::available("kotlinc", "-version")
        || !ManagedCapabilityCheck::available("java", "-version")
    {
        eprintln!("skipping: Kotlin JVM tools are unavailable");
        return;
    }
    let output = KotlinPageGraphCheck::run(KotlinPageGraphCheck::CAPACITY_CALLER);
    println!("{output}");
    assert!(output.contains("PASS kotlin real capability capacity 256"));
}

#[test]
fn kotlin_native_page_node_guard_handles_synthetic_retained_capacity() {
    if !ManagedCapabilityCheck::available("kotlinc", "-version")
        || !ManagedCapabilityCheck::available("java", "-version")
    {
        eprintln!("skipping: Kotlin JVM tools are unavailable");
        return;
    }
    let output = KotlinPageGraphCheck::run(KotlinPageGraphCheck::GUARD_CALLER);
    println!("{output}");
    assert!(output.contains("PASS kotlin synthetic page-entry guard unit 4096"));
}

struct KotlinListenerCheck;

impl KotlinListenerCheck {
    const MODEL: &'static str = "data class Ping(val step: Int)\n";
    const SERVICE: &'static str = "@Service\nclass HeardService {\n    val applicationEventPublisher: ApplicationEventPublisher\n    var heard: String = \"\"\n    fun send(step: Int) { applicationEventPublisher.publishEvent(Ping(step)) }\n    @OnEvent\n    suspend fun later(ping: Ping) { heard += \"a\" + Steps.of(ping) }\n    @OnEvent\n    fun now(ping: Ping) { heard += \"s\" + ping.step }\n}\n";
    const CALLER: &'static str = r#"
data class Ping(val step: Int)

object Steps {
    suspend fun of(ping: Ping): Int = ping.step
}

fun heardAsk(op: String, extra: String = ""): String =
    tacInvokeOwned("{\"route\":\"/\",\"service\":\"heard\",\"owner\":\"00000000-0000-4000-8000-000000000091\",\"op\":\"$op\"$extra}")

fun main() {
    val init = heardAsk("init")
    check(!init.contains("error")) { init }
    check(heardAsk("call", ",\"name\":\"send\",\"args\":[1]") == "{\"value\":null}") { "the first send failed" }
    val first = heardAsk("get", ",\"name\":\"heard\"")
    check(first == "{\"value\":\"a1s1\"}") { "the suspend listener did not finish before the plain one: $first" }
    check(heardAsk("call", ",\"name\":\"send\",\"args\":[2]") == "{\"value\":null}") { "the second send failed" }
    val second = heardAsk("get", ",\"name\":\"heard\"")
    check(second == "{\"value\":\"a1s1a2s2\"}") { "the listeners lost their order: $second" }
    check(tacInvokeOwned("{\"op\":\"host-dispose\"}") == "{\"value\":null}")
    println("PASS kotlin native suspend listeners run in order")
}
"#;

    fn run() -> String {
        let root = tempfile::tempdir().expect("project");
        let models = root.path().join("client/models");
        let services = root.path().join("client/services");
        std::fs::create_dir_all(&models).expect("models");
        std::fs::create_dir_all(&services).expect("services");
        std::fs::write(models.join("Ping.kt"), Self::MODEL).expect("model");
        let path = services.join("HeardService.kt");
        std::fs::write(&path, Self::SERVICE).expect("service source");
        let input = NativeCompanionInput {
            snapshot_root: root.path().to_path_buf(),
            language: NativeCompanion::Kotlin,
            source: path,
            service: String::from("heard"),
            class: String::from("HeardService"),
            web_assembly: false,
            application_provider: false,
        };
        let registry = super::registry::source(&[input], NativeCompanion::Kotlin)
            .expect("registry")
            .expect("source");
        ManagedCapabilityCheck::kotlin_jvm(root.path(), &registry, Self::CALLER, true)
    }
}

#[test]
fn kotlin_native_suspend_listeners_run_in_source_order() {
    if !ManagedCapabilityCheck::available("kotlinc", "-version")
        || !ManagedCapabilityCheck::available("java", "-version")
    {
        eprintln!("skipping: Kotlin JVM tools are unavailable");
        return;
    }
    let output = KotlinListenerCheck::run();
    println!("{output}");
    assert!(output.contains("PASS kotlin native suspend listeners run in order"));
}
