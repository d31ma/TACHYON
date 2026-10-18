#![allow(clippy::expect_used)]

use super::{LuaRuntimeBuild, LuaWebServices};
use std::path::{Path, PathBuf};

struct Page;

impl Page {
    const FILE_FETCH: &str = "import { readFile } from 'node:fs/promises'\nglobalThis.fetch = async (url) => new Response(await readFile(url))\n";

    const COUNTER: &str = "-- @Service\n-- @Lifetime({\"value\":\"prototype\"})\n-- @NeedsAll\n---@class CounterService\nCounterService = {\n  ---@type integer\n  count = 0,\n  ---@type string\n  label = \"\",\n  ---@type string[]\n  notes = {},\n}\n\n---@return integer\nfunction CounterService:increment()\n  self.count = self.count + 1\n  return self.count\nend\n\n---@return integer\nfunction CounterService:doubled()\n  return self.count * 2\nend\n\n---@param by integer\n---@param why string\n---@return string\nfunction CounterService:add(by, why)\n  self.count = self.count + by\n  self.notes[#self.notes + 1] = why\n  return why .. \" \" .. self:_total()\nend\n\n---@return integer\nfunction CounterService:_total()\n  return self.count\nend\n";

    const COUNTER_DRIVER: &str = "const { default: CounterService } = await import('./CounterService.js')\n\
         const one = new CounterService({ label: 'many', ignored: 1 }), two = new CounterService({})\n\
         one.increment(); one.increment(); two.increment()\n\
         const doubled = one.doubled()\n\
         one.count = 40\n\
         const after = one.increment()\n\
         const added = one.add(new Event('click'), 9, 'now')\n\
         let bigCall = ''\n\
         try { one.add(new Event('click'), 1, 'x'.repeat(70000)) } catch (error) { bigCall = error.message }\n\
         const hidden = typeof one._total\n\
         const notes = [one.notes, two.notes]\n\
         const labels = [one.label, two.label]\n\
         one.__tachyonDispose()\n\
         let stale = false\n\
         try { one.count } catch { stale = true }\n\
         console.log(JSON.stringify({ doubled, after, other: two.count, added, bigCall, hidden, notes, labels, stale, keys: Object.keys(two) }))\n\
         process.exit(0)\n";

    const EVENT_COUNTER: &str = "-- @Service\n-- @Lifetime({\"value\":\"prototype\"})\n-- @NeedsAll\n---@class TallyService\nTallyService = {\n  ---@type integer\n  count = 0,\n  ---@type string\n  label = \"\",\n  ---@type ApplicationEventPublisher\n  application_event_publisher = nil,\n}\n\n---@return nil\n-- @OnStart\nfunction TallyService:start()\n  self.label = self.label .. \" clicks\"\n  self.application_event_publisher:publish_event(setmetatable({ label = self.label }, CounterStarted))\nend\n\n---@return integer\nfunction TallyService:increment()\n  self.count = self.count + 1\n  self.application_event_publisher:publish_event(setmetatable({ count = self.count }, CounterAdded))\n  return self.count\nend\n\n---@return nil\n-- @OnStop\nfunction TallyService:stop()\n  print(\"stopped \" .. self.count)\nend\n";

    const EVENT_COUNTER_DRIVER: &str = "const { default: TallyService, __tachyon_event_plan__: plan } = await import('./TallyService.js')\n\
         const events = []\n\
         const one = new TallyService({ label: 'many' }), two = new TallyService({})\n\
         for (const [tag, owner] of [['one', one], ['two', two]]) {\n\
         \x20 Object.defineProperty(owner, '__tachyonPublisher', { value: { publishEvent: (event, value) => events.push([tag, event, value]) } })\n\
         \x20 owner.__tachyonStarted()\n\
         }\n\
         one.increment(); two.increment()\n\
         const keys = Object.keys(one)\n\
         one.__tachyonDispose()\n\
         console.log(JSON.stringify({ events, keys, label: two.label }))\n\
         process.exit(0)\n";

    const SIGNAL: &str = "-- @Service\n---@class SignalService\nSignalService = {\n  ---@type integer[]\n  large = {},\n  ---@type integer\n  echoed = 0,\n  ---@type ApplicationEventPublisher\n  application_event_publisher = nil,\n}\n\n---@param size integer\n---@return nil\nfunction SignalService:notify(size)\n  self.application_event_publisher:publish_event(setmetatable({ size = size }, SignalSent))\nend\n\n---@param sent SignalSent\n---@return SignalEcho\n-- @OnEvent({\"condition\":\"value.size > 5\"})\nfunction SignalService:record(sent)\n  self.large[#self.large + 1] = sent.size\n  return setmetatable({ value = sent.size * 10 }, SignalEcho)\nend\n\n---@param reply SignalEcho\n---@return nil\n-- @OnEvent\n-- @Order(1)\nfunction SignalService:echo(reply)\n  self.echoed = reply.value\nend\n";

    const SHELF: &str = "-- @Service\n---@class ShelfService\nShelfService = {\n  ---@type integer[]\n  heard = {},\n}\n\n---@param changed ShelfChanged\n---@return nil\n-- @OnEvent\nfunction ShelfService:changed(changed)\n  self.heard[#self.heard + 1] = changed.count\nend\n";

    const SHELF_DRIVER: &str = "const sources = []\n\
         globalThis.EventSource = class { constructor(url) { this.url = url; sources.push(this) } addEventListener() {} close() {} }\n\
         const { default: ShelfService, __tachyon_event_plan__: plan } = await import('./ShelfService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const shelf = new ShelfService({})\n\
         bus.attach(shelf, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(shelf)\n\
         for (const source of sources) source.onmessage({ data: JSON.stringify({ count: 3 }), lastEventId: '1' })\n\
         console.log(JSON.stringify({ heard: shelf.heard, topics: sources.map((source) => source.url) }))\n\
         process.exit(0)\n";

    const MODELS: [(&str, &str); 5] = [
        (
            "client/models/CounterStarted.lua",
            "---@class CounterStarted\n---@field label string\nCounterStarted = {}\n",
        ),
        (
            "client/models/CounterAdded.lua",
            "---@class CounterAdded\n---@field count integer\nCounterAdded = {}\n",
        ),
        (
            "client/models/SignalSent.lua",
            "---@class SignalSent\n---@field size integer\nSignalSent = {}\n",
        ),
        (
            "client/models/SignalEcho.lua",
            "---@class SignalEcho\n---@field value integer\nSignalEcho = {}\n",
        ),
        (
            "client/models/ShelfChanged.lua",
            "-- @Topic\n---@class ShelfChanged\n---@field count integer\nShelfChanged = {}\n",
        ),
    ];

    fn event_runtime(root: &Path) {
        std::fs::write(
            root.join("application-events.mjs"),
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        )
        .expect("event runtime");
    }

    const SIGNAL_DRIVER: &str = "const { default: SignalService, __tachyon_event_plan__: plan } = await import('./SignalService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const events = []\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         for (const event of ['SignalSent', 'SignalEcho']) bus.attach({ receive: (value) => events.push([event, value]) }, { listeners: [{ method: 'receive', event, order: -100 }] }, {})\n\
         const signal = new SignalService({})\n\
         bus.attach(signal, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(signal)\n\
         signal.notify(3); signal.notify(8)\n\
         console.log(JSON.stringify({ large: signal.large, echoed: signal.echoed, events }))\n\
         process.exit(0)\n";

    fn node_available() -> bool {
        let found = std::process::Command::new("node")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success());
        assert!(
            found || std::env::var_os("CI").is_none(),
            "CI must provide node"
        );
        found
    }

    fn skip_reason() -> Option<String> {
        if !Self::node_available() {
            return Some(String::from("node is unavailable"));
        }
        LuaRuntimeBuild::unavailable_reason()
    }

    fn write(root: &Path, relative: &str, text: &str) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, text).expect("source");
        path
    }

    async fn publish(root: &Path, services: &[(&str, &str, &str)]) {
        for (relative, text) in Self::MODELS {
            Self::write(root, relative, text);
        }
        let mut chosen = Vec::new();
        for (instance, class, source) in services {
            let path = Self::write(root, &format!("client/services/{class}.lua"), source);
            chosen.push((
                String::from(*instance),
                String::from(*class),
                crate::tac_web::WebLanguage::Lua,
                path,
            ));
        }
        let jobs = crate::tac_web::BuildJobs::new(1);
        let published =
            crate::tac_web::build_all(root, &chosen, &jobs, crate::BuildProfile::Release)
                .await
                .expect("build");
        for (name, bytes) in published.files {
            Self::write_bytes(&root.join(name), &bytes);
        }
    }

    fn write_bytes(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, bytes).expect("publish");
    }

    fn run(root: &Path, driver: &str) -> serde_json::Value {
        std::fs::write(
            root.join("drive.mjs"),
            format!("{}{driver}", Self::FILE_FETCH),
        )
        .expect("driver");
        let output = std::process::Command::new("node")
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
        let last = stdout.lines().last().unwrap_or_default();
        serde_json::from_str(last).expect("report")
    }
}

#[test]
fn a_lua_service_registers_its_public_members_after_the_prelude() {
    let root = tempfile::tempdir().expect("project");
    let service = Page::write(
        root.path(),
        "client/services/CounterService.lua",
        Page::COUNTER,
    );
    Page::write(
        root.path(),
        "client/models/Tally.lua",
        "---@class Tally\n---@field total integer\nTally = {}\n",
    );
    let program = LuaWebServices::program(
        root.path(),
        &[(
            String::from("counter"),
            String::from("CounterService"),
            service,
        )],
    )
    .expect("program");
    assert!(program.starts_with(LuaWebServices::PRELUDE), "{program}");
    assert!(program.contains("Tally.__name = \"Tally\""), "{program}");
    let registration = program
        .lines()
        .find(|line| line.starts_with("TacServices.register([[counter]], CounterService, [["))
        .expect("registration");
    for expected in [
        "\"fields\":[\"count\",\"label\",\"notes\"]",
        "\"writable\":[\"count\",\"label\",\"notes\"]",
        "\"methods\":[\"increment\",\"doubled\",\"add\"]",
        "\"required\":[\"*\"]",
    ] {
        assert!(
            registration.contains(expected),
            "{expected}: {registration}"
        );
    }
}

#[test]
fn a_behavior_annotation_on_a_lua_service_wraps_the_method_for_the_page() {
    let root = tempfile::tempdir().expect("project");
    let service = Page::write(
        root.path(),
        "client/services/PriceService.lua",
        "-- @Service\n---@class PriceService\nPriceService = {}\n\n---@param sku string\n---@return integer\n-- @Retry({\"maxRetries\":3,\"delayMs\":5,\"multiplier\":1,\"maxDelayMs\":5,\"jitterMs\":0})\nfunction PriceService:price(sku)\n  return #sku\nend\n",
    );
    let program = LuaWebServices::program(
        root.path(),
        &[(String::from("price"), String::from("PriceService"), service)],
    )
    .unwrap_or_else(|failure| panic!("{failure}"));
    assert!(
        program.contains("function PriceService:price(sku) return self.behaviors:invoke(self, \"PriceService\", \"price\", table.pack(sku), function(sku)"),
        "{program}"
    );
    assert!(program.contains("Behaviors.register([["), "{program}");
    assert!(program.contains("]], \"tac\", "), "{program}");
}

#[test]
fn a_lua_service_refuses_a_native_capability_field_instead_of_leaving_it_nil() {
    for capability in ["WindowControls", "FilePicker"] {
        let root = tempfile::tempdir().expect("project");
        let service = Page::write(
            root.path(),
            "client/services/ReportService.lua",
            &format!(
                "-- @Service\n---@class ReportService\nReportService = {{\n  -- @Needs\n  ---@type {capability}\n  provider = nil,\n}}\n"
            ),
        );
        let failure = LuaWebServices::program(
            root.path(),
            &[(
                String::from("report"),
                String::from("ReportService"),
                service,
            )],
        )
        .expect_err("a Lua service refuses a native capability field");
        let text = failure.to_string();
        assert!(text.contains("TY2015"), "{text}");
        assert!(
            text.contains(&format!("do not support {capability}")),
            "{text}"
        );
        assert!(text.contains("ReportService.lua"), "{text}");
    }
}

#[tokio::test]
async fn a_lua_service_answers_the_page_from_the_lua_runtime() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    Page::publish(root.path(), &[("counter", "CounterService", Page::COUNTER)]).await;
    let report = Page::run(root.path(), Page::COUNTER_DRIVER);
    assert_eq!(report["doubled"], 4, "{report}");
    assert_eq!(report["after"], 41, "{report}");
    assert_eq!(report["other"], 1, "{report}");
    assert_eq!(report["added"], "now 50", "{report}");
    assert_eq!(
        report["bigCall"],
        "Web service counter: The request to add is 70121 bytes. The limit is 65536 bytes. Pass less data to the service."
    );
    assert_eq!(report["hidden"], "undefined", "{report}");
    assert_eq!(
        report["notes"],
        serde_json::json!([["now"], []]),
        "{report}"
    );
    assert_eq!(
        report["labels"],
        serde_json::json!(["many", ""]),
        "{report}"
    );
    assert_eq!(report["stale"], true, "{report}");
    assert_eq!(
        report["keys"],
        serde_json::json!(["count", "label", "notes"])
    );
}

#[tokio::test]
async fn the_lua_runtime_is_built_once_into_the_user_cache_and_never_into_the_project() {
    if let Some(reason) = LuaRuntimeBuild::unavailable_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let files = LuaWebServices::runtime_files().await.expect("runtime");
    let runtime = &files[0].1;
    assert!(runtime.starts_with(b"\0asm"));
    assert!(runtime.len() < 1024 * 1024, "{}", runtime.len());
    assert_eq!(files[1].0, "lua/tachyon-lua.js");
    let cached = LuaRuntimeBuild::cached_path();
    assert!(
        cached.starts_with(crate::cache::root()),
        "{}",
        cached.display()
    );
    assert_eq!(runtime, &std::fs::read(&cached).expect("cached"));
}

#[tokio::test]
async fn a_lua_service_starts_publishes_and_stops_through_the_page() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    Page::publish(
        root.path(),
        &[("tally", "TallyService", Page::EVENT_COUNTER)],
    )
    .await;
    let report = Page::run(root.path(), Page::EVENT_COUNTER_DRIVER);
    assert_eq!(
        report["events"],
        serde_json::json!([
            ["one", "CounterStarted", {"label": "many clicks"}],
            ["two", "CounterStarted", {"label": " clicks"}],
            ["one", "CounterAdded", {"count": 1}],
            ["two", "CounterAdded", {"count": 1}],
        ]),
        "{report}"
    );
    assert_eq!(report["keys"], serde_json::json!(["count", "label"]));
    assert_eq!(report["label"], " clicks");
}

#[tokio::test]
async fn lua_listeners_follow_their_conditions_and_order() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    Page::publish(root.path(), &[("signal", "SignalService", Page::SIGNAL)]).await;
    Page::event_runtime(root.path());
    let report = Page::run(root.path(), Page::SIGNAL_DRIVER);
    assert_eq!(report["large"], serde_json::json!([8]), "{report}");
    assert_eq!(report["echoed"], 80, "{report}");
    assert_eq!(
        report["events"],
        serde_json::json!([
            ["SignalSent", {"size": 3}],
            ["SignalSent", {"size": 8}],
            ["SignalEcho", {"value": 80}],
        ])
    );
}

#[tokio::test]
async fn a_lua_listener_hears_its_topic_stream() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    Page::publish(root.path(), &[("shelf", "ShelfService", Page::SHELF)]).await;
    Page::event_runtime(root.path());
    let report = Page::run(root.path(), Page::SHELF_DRIVER);
    assert_eq!(report["heard"], serde_json::json!([3]), "{report}");
    assert_eq!(
        report["topics"],
        serde_json::json!(["/.tachyon/topics/shelf-changed"])
    );
}

#[test]
fn the_tac_lua_runtime_passes_its_contract() {
    let Some(printed) = crate::lua_source::LuaProgram::printed(&[
        LuaWebServices::PRELUDE,
        include_str!("../lua_source/lua_behaviors.lua"),
        include_str!("../../tests/fixtures/lua/tac_runtime_contract.lua"),
    ]) else {
        return;
    };
    assert!(
        printed.contains("Tac Lua runtime contracts passed"),
        "{printed}"
    );
}

struct NeededServicePage;

impl NeededServicePage {
    const COUNTER: &str = "// @Service\nexport default class CounterService {\n  count = 0\n\n  increment() {\n    this.count += 1\n  }\n}\n";
    const TALLY: &str = "-- @Service\n---@class TallyService\nTallyService = {\n  -- @Needs\n  ---@type CounterService\n  counter = nil,\n}\n\n---@return nil\nfunction TallyService:bump()\n  self.counter:increment()\nend\n";
    const MESSAGE: &str = "The `-- @Needs` field 'counter' names CounterService, another Tac service. A Lua Tac service cannot need another service yet (ADR 0085): the page does not fill the field, so it would stay nil and the first call through it would fail.";
}

#[tokio::test]
async fn a_lua_service_that_needs_another_service_stops_the_build_at_the_field() {
    let root = tempfile::tempdir().expect("project");
    Page::write(
        root.path(),
        "client/services/CounterService.js",
        NeededServicePage::COUNTER,
    );
    let tally = Page::write(
        root.path(),
        "client/services/TallyService.lua",
        NeededServicePage::TALLY,
    );
    let chosen = [(
        String::from("tally"),
        String::from("TallyService"),
        crate::tac_web::WebLanguage::Lua,
        tally,
    )];
    let jobs = crate::tac_web::BuildJobs::new(1);
    let built =
        crate::tac_web::build_all(root.path(), &chosen, &jobs, crate::BuildProfile::Release).await;
    let Err(failure) = built else {
        panic!("the build accepted a Lua service that needs another service");
    };
    assert_eq!(failure.diagnostics().len(), 1, "{failure}");
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code.to_string(), "TY2015", "{failure}");
    assert_eq!(diagnostic.message, NeededServicePage::MESSAGE, "{failure}");
    assert!(
        diagnostic
            .help
            .as_deref()
            .is_some_and(|help| help
                .contains("A Lua Needs field takes a Store, a Converter or an HTTPClient")),
        "{failure}"
    );
    let span = &diagnostic.spans[0];
    assert_eq!(span.file, "client/services/TallyService.lua", "{failure}");
    let start = usize::try_from(span.start).expect("start");
    let end = usize::try_from(span.end).expect("end");
    assert_eq!(&NeededServicePage::TALLY[start..end], "  counter = nil,");
}
