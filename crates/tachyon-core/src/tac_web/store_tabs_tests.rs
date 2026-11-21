#![allow(clippy::expect_used)]

use crate::node_driver::NodeDriver;
use std::path::Path;

struct StoreTabsDriver;

impl StoreTabsDriver {
    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
console.info = () => {}
const channels = []
const posted = []
globalThis.BroadcastChannel = class { constructor(name) { this.name = name; channels.push(this) } postMessage(message) { posted.push(structuredClone(message)) } }
;(0, eval)(`${await readFile('./memory-indexeddb.js', 'utf8')}\nglobalThis.MemoryIndexedDb = MemoryIndexedDb`)
MemoryIndexedDb.install()
MemoryIndexedDb.seed('tachyon-store', 'notes', 'id', '', [{ id: 'n1', text: 'one' }])
MemoryIndexedDb.seed('tachyon-store', 'messages', 'id', 'room', [{ id: 'm1', room: 'r1' }, { id: 'm2', room: 'r1' }, { id: 'm3', room: 'r2' }])
globalThis.document = { visibilityState: 'visible', addEventListener: () => {}, querySelector: () => null }
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
;(0, eval)(await readFile('./tac-storage.js', 'utf8'))
const published = []
TacWebApis.events = { publish: (name, value) => published.push([name, value]), send: () => {} }
const storage = globalThis[Symbol.for('tachyon.tac.storage')]
await storage.mirror()
const settle = async () => { for (let round = 0; round < 40; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const host = (command) => JSON.parse(storage.host(JSON.stringify({ op: 'entity', command })))
const note = (operation, extra = {}) => host({ operation, table: 'notes', idField: 'id', entity: 'Note', ...extra })
const message = (operation, extra = {}) => host({ operation, table: 'messages', idField: 'id', entity: 'Message', partition: 'room', ...extra })
const texts = () => note('find', { page: { size: 50, offset: 0, sort: [] } }).value.map((row) => row.text)
const room = (value) => message('query', { query: { kind: 'find', where: [{ field: 'room', operator: 'eq', value }], orderBy: [], page: { size: 50, offset: 0, sort: [] } } }).value.map((row) => row.id)
const table = (name) => MemoryIndexedDb.databases.get('tachyon-store').tables.get(name).rows
const hear = (changes) => channels[0].onmessage({ data: { changes } })
const report = { channels: channels.map((channel) => channel.name) }
report.before = texts()
table('notes').set('n1', { id: 'n1', text: 'changed' })
table('notes').set('n2', { id: 'n2', text: 'two' })
hear([{ table: 'notes', ids: ['n1', 'n2'] }, { table: 'unknown', ids: ['x'] }, { table: 5 }, 'junk'])
await settle()
report.eager = texts()
report.eagerEvents = published.splice(0)
table('notes').set('n1', { id: 'n1', text: 'theirs' })
hear([{ table: 'notes', ids: ['n1'] }])
note('save', { value: { id: 'n1', text: 'mine' } })
await settle()
report.localWins = { memory: texts(), stored: table('notes').get('n1').text, events: published.splice(0) }
report.posted = posted.splice(0)
message('load', { value: 'r1' })
await settle()
published.splice(0)
report.loaded = room('r1')
table('messages').set('m2', { id: 'm2', room: 'r2' })
table('messages').delete('m1')
table('messages').set('m4', { id: 'm4', room: 'r1' })
hear([{ table: 'messages', ids: ['m1', 'm2', 'm4'] }])
await settle()
report.moved = { r1: room('r1'), events: published.splice(0) }
await storage.entities().execute({ operation: 'save', table: 'notes', idField: 'id', value: { id: 'n9', text: 'from javascript' } })
await storage.entities().execute({ operation: 'transaction', action: 'begin' }).then(({ transaction }) => storage.entities().execute({ operation: 'delete', table: 'notes', idField: 'id', id: 'n2', transaction }).then(() => storage.entities().execute({ operation: 'transaction', action: 'commit', transaction })))
await settle()
report.javascript = { posted: posted.splice(0), memory: texts() }
console.log(JSON.stringify(report))
process.exit(0)
";

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        let files = [
            ("tac-storage.js", include_str!("../template/tac-storage.js")),
            ("tac-web.js", include_str!("../template/tac-web.js")),
            ("memory-indexeddb.js", include_str!("memory_indexeddb.js")),
            ("drive.mjs", Self::DRIVER),
        ];
        for (name, text) in files {
            Self::write(root.path(), name, text);
        }
        let mut command = NodeDriver::command();
        command.arg("drive.mjs").current_dir(root.path());
        let output = NodeDriver::output(&mut command).expect("node drive.mjs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.lines().last().unwrap_or_default()).expect("report")
    }

    fn write(root: &Path, name: &str, text: &str) {
        std::fs::write(root.join(name), text).expect("write");
    }

    fn changed(table: &str, partition: &str) -> serde_json::Value {
        serde_json::json!(["StoreChanged", {"table": table, "partition": partition}])
    }
}

#[test]
fn another_tab_changes_reach_an_eager_table_and_its_services() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = StoreTabsDriver::run();
    assert_eq!(report["channels"], serde_json::json!(["tachyon-store"]));
    assert_eq!(report["before"], serde_json::json!(["one"]));
    assert_eq!(report["eager"], serde_json::json!(["changed", "two"]));
    assert_eq!(
        report["eagerEvents"],
        serde_json::json!([StoreTabsDriver::changed("notes", "")]),
        "unknown tables and broken messages are ignored: {report}"
    );
}

#[test]
fn a_local_write_that_is_not_stored_yet_wins_over_another_tab() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = StoreTabsDriver::run();
    assert_eq!(
        report["localWins"],
        serde_json::json!({"memory": ["mine", "two"], "stored": "mine", "events": []}),
        "{report}"
    );
    assert_eq!(
        report["posted"],
        serde_json::json!([{"changes": [{"table": "notes", "ids": ["n1"]}]}])
    );
}

#[test]
fn another_tab_changes_move_rows_between_loaded_partitions() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = StoreTabsDriver::run();
    assert_eq!(report["loaded"], serde_json::json!(["m1", "m2"]));
    assert_eq!(
        report["moved"],
        serde_json::json!({
            "r1": ["m4"],
            "events": [
                StoreTabsDriver::changed("messages", "r1"),
                StoreTabsDriver::changed("messages", "r2"),
            ],
        }),
        "{report}"
    );
}

#[test]
fn javascript_adapter_writes_reach_the_other_tabs() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = StoreTabsDriver::run();
    assert_eq!(
        report["javascript"],
        serde_json::json!({
            "posted": [
                {"changes": [{"table": "notes", "ids": ["n9"]}]},
                {"changes": [{"table": "notes", "ids": ["n2"]}]},
            ],
            "memory": ["mine", "from javascript"],
        }),
        "{report}"
    );
}
