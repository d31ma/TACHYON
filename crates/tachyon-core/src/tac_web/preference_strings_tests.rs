#![allow(clippy::expect_used)]

use std::time::Duration;

struct PreferenceHostFixture;

impl PreferenceHostFixture {
    const RUNTIME: &'static str = concat!(
        include_str!("../template/application-events.js"),
        "\n",
        include_str!("../template/tac-preference-bounds.js"),
        "\n",
        include_str!("../template/tac-preference-strings.js"),
        "\n",
        include_str!("../template/tac-web.js"),
        "\n"
    );
    const BEFORE: &'static str = r"let granted = false;
let denied = false;
let calls = 0;
const values = new Map();
globalThis.document = { querySelector: () => ({ content: granted ? 'preferenceStrings' : '' }) };
globalThis.localStorage = {
  getItem(key) { calls++; if (denied) throw new Error('denied'); return values.get(key) ?? null; },
  setItem(key, value) { calls++; if (denied) throw new Error('denied'); values.set(key, value); },
};
";
    const AFTER: &'static str = r"const ask = (call, args) => JSON.parse(globalThis[Symbol.for('tachyon.tac.web')].host(JSON.stringify({ api: 'preferenceStrings', call, args }))).value;
const report = {};
report.ungrantedRead = ask('read', ['tachyon.platform']);
report.ungrantedWrite = ask('write', ['tachyon.platform', 'macOS']);
report.ungrantedCalls = calls;
granted = true;
report.missing = ask('read', ['tachyon.platform']);
report.written = ask('write', ['tachyon.platform', 'macOS']);
report.raw = values.get('tachyon.platform');
report.read = ask('read', ['tachyon.platform']);
report.exactLimit = ask('write', ['x'.repeat(128), 'y'.repeat(4096)]);
report.longKey = ask('write', ['x'.repeat(129), 'ok']);
report.byteKey = ask('write', ['é'.repeat(65), 'ok']);
report.longValue = ask('write', ['bounded', 'y'.repeat(4097)]);
report.byteValue = ask('write', ['bounded', 'é'.repeat(2049)]);
report.emptyKey = ask('write', ['', 'ok']);
report.number = ask('write', ['bounded', 4]);
values.set('oversized', 'x'.repeat(4097));
report.oversizedRead = ask('read', ['oversized']);
denied = true;
report.deniedRead = ask('read', ['tachyon.platform']);
report.deniedWrite = ask('write', ['tachyon.platform', 'Linux']);
denied = false;
const events = new TacApplicationEvents({ calling: (owner, work) => work() });
const first = {};
events.attach(first, { resources: [{ field: 'port', name: 'preferenceStrings' }] }, { preferenceStrings: TacWebApis.preferenceStrings });
const retained = first.port;
events.dispose(first);
const second = {};
events.attach(second, { resources: [{ field: 'port', name: 'preferenceStrings' }] }, { preferenceStrings: TacWebApis.preferenceStrings });
const before = calls;
try { retained.write('tachyon.platform', 'stale'); report.disposed = false; }
catch (error) { report.disposed = error.message === 'Application owner disposed.'; }
report.disposedCalls = calls - before;
report.secondWrite = second.port.write('tachyon.platform', 'Linux');
report.secondRead = second.port.read('tachyon.platform');
console.log(JSON.stringify(report));
";

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("project");
        let program = format!("{}{}{}", Self::BEFORE, Self::RUNTIME, Self::AFTER);
        let script = root.path().join("preferences.mjs");
        std::fs::write(&script, program).expect("script");
        let output = crate::node_driver::NodeDriver::output_within(
            crate::node_driver::NodeDriver::command().arg(script),
            Duration::from_secs(20),
        )
        .expect("supervised Node");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("report")
    }
}

#[test]
fn preference_strings_preserve_raw_storage_and_enforce_grants_byte_bounds_and_disposal() {
    let report = PreferenceHostFixture::run();
    for name in ["ungrantedRead", "missing", "oversizedRead", "deniedRead"] {
        assert!(report[name].is_null(), "{name}: {report}");
    }
    for name in [
        "ungrantedWrite",
        "longKey",
        "byteKey",
        "longValue",
        "byteValue",
        "emptyKey",
        "number",
        "deniedWrite",
    ] {
        assert_eq!(report[name], false, "{name}: {report}");
    }
    for name in ["written", "exactLimit", "disposed", "secondWrite"] {
        assert_eq!(report[name], true, "{name}: {report}");
    }
    assert_eq!(report["ungrantedCalls"], 0);
    assert_eq!(report["disposedCalls"], 0);
    assert_eq!(report["raw"], "macOS");
    assert_eq!(report["read"], "macOS");
    assert_eq!(report["secondRead"], "Linux");
}

#[test]
fn javascript_and_typescript_preference_resources_use_the_existing_needs_registration() {
    for (path, source) in [
        (
            "client/services/ChoiceService.js",
            "class ChoiceService {\n @Needs\n preferenceStrings;\n read() { return this.preferenceStrings.read('tachyon.platform'); }\n}\n",
        ),
        (
            "client/services/ChoiceService.ts",
            "class ChoiceService {\n @Needs\n preferenceStrings: PreferenceStrings;\n read(): string | null { return this.preferenceStrings.read('tachyon.platform'); }\n}\n",
        ),
    ] {
        let module = crate::events::extract(std::path::Path::new(path), source, "tac")
            .expect("resource metadata")
            .expect("module");
        let resource = &module.types[0].resources[0];
        assert_eq!(resource.field, "preferenceStrings");
        assert_eq!(resource.name, "preferenceStrings");
        assert!(!module.source.contains("@Needs"));
    }
}
