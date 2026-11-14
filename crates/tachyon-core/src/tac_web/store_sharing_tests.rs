#![allow(clippy::expect_used)]

use crate::node_driver::NodeDriver;
use std::path::Path;

struct StoreSharingDriver;

impl StoreSharingDriver {
    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
;(0, eval)(`${await readFile('./memory-indexeddb.js', 'utf8')}\nglobalThis.MemoryIndexedDb = MemoryIndexedDb`)
MemoryIndexedDb.install()
globalThis.document = { visibilityState: 'visible', addEventListener: () => {}, querySelector: () => null }
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
;(0, eval)(`${await readFile('./tac-storage.js', 'utf8')}\nglobalThis.TacEntityStorage = TacEntityStorage`)
const storage = globalThis.TacEntityStorage
const page = (app) => { storage.app = app; storage.current = null; storage.checked = ''; storage.wanted = new Map(); storage.queue = Promise.resolve() }
const save = (table, partition, value) => storage.adapter.execute({ operation: 'save', table, idField: 'id', partition, value })
const failure = async (work) => { try { await work(); return '' } catch (error) { return `${error.code}: ${error.message}` } }
const tables = () => MemoryIndexedDb.databases.get('tachyon-store').tables
const realOpen = indexedDB.open
const raceOnce = (change) => { indexedDB.open = (name, version) => { if (version !== undefined) { indexedDB.open = realOpen; change(MemoryIndexedDb.databases.get(name), version) } return realOpen(name, version) } }
const answers = {}
page('/chat/')
await save('messages', 'room', { id: 'm1', room: 'r1' })
page('/mail/')
answers.sameShape = await failure(() => save('messages', 'room', { id: 'm2', room: 'r2' }))
answers.owners = MemoryIndexedDb.rows('tachyon-store', 'tachyon:tables')
page('/search/')
answers.conflict = await failure(() => save('messages', 'thread', { id: 'm3', thread: 't1' }))
answers.conflictIndex = tables().get('messages').indexes.get('tachyon:partition')
page('/notes/')
await save('notes', 'folder', { id: 'n1', folder: 'f1' })
page('/notes/')
answers.ownChange = await failure(() => save('notes', 'owner', { id: 'n2', owner: 'o1' }))
answers.ownChangeIndex = tables().get('notes').indexes.get('tachyon:partition')
page('/race/')
raceOnce((database, version) => { database.version = version; database.tables.set('other', { keyPath: 'id', rows: new Map(), indexes: new Map() }) })
answers.race = await failure(() => save('drafts', '', { id: 'd1' }))
answers.raceRows = MemoryIndexedDb.rows('tachyon-store', 'drafts').map((row) => row.id)
page('/ahead/')
raceOnce((database, version) => { database.version = version + 5 })
answers.ahead = await failure(() => save('outbox', '', { id: 'o1' }))
answers.aheadRows = MemoryIndexedDb.rows('tachyon-store', 'outbox').map((row) => row.id)
console.log(JSON.stringify(answers))
";

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        Self::write(
            root.path(),
            "memory-indexeddb.js",
            include_str!("memory_indexeddb.js"),
        );
        Self::write(
            root.path(),
            "tac-web.js",
            include_str!("../template/tac-web.js"),
        );
        Self::write(
            root.path(),
            "tac-storage.js",
            include_str!("../template/tac-storage.js"),
        );
        Self::write(root.path(), "drive.mjs", Self::DRIVER);
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
}

#[test]
fn two_apps_that_declare_a_table_the_same_way_share_it() {
    let answers = StoreSharingDriver::run();
    assert_eq!(answers["sameShape"], "");
}

#[test]
fn the_table_register_names_every_app_that_uses_a_table() {
    let answers = StoreSharingDriver::run();
    assert_eq!(
        answers["owners"],
        serde_json::json!([{ "table": "messages", "id": "id", "partition": "room", "apps": ["/chat/", "/mail/"] }])
    );
}

#[test]
fn another_app_cannot_split_a_shared_table_by_another_field() {
    let answers = StoreSharingDriver::run();
    let conflict = answers["conflict"].as_str().expect("conflict");
    assert!(
        conflict.starts_with("storage_table_conflict: The page app at /search/"),
        "{conflict}"
    );
}

#[test]
fn a_refused_partition_keeps_the_stored_index() {
    let answers = StoreSharingDriver::run();
    assert_eq!(answers["conflictIndex"], "room");
}

#[test]
fn the_only_app_of_a_table_may_change_its_partition_field() {
    let answers = StoreSharingDriver::run();
    assert_eq!(answers["ownChange"], "");
}

#[test]
fn a_changed_partition_field_rebuilds_the_index() {
    let answers = StoreSharingDriver::run();
    assert_eq!(answers["ownChangeIndex"], "owner");
}

#[test]
fn a_tab_that_upgrades_at_the_same_time_does_not_hide_a_new_table() {
    let answers = StoreSharingDriver::run();
    assert_eq!(answers["race"], "");
}

#[test]
fn a_row_is_stored_after_an_upgrade_race() {
    let answers = StoreSharingDriver::run();
    assert_eq!(answers["raceRows"], serde_json::json!(["d1"]));
}

#[test]
fn a_tab_that_is_versions_ahead_does_not_stop_the_upgrade() {
    let answers = StoreSharingDriver::run();
    assert_eq!(answers["ahead"], "");
}

#[test]
fn a_row_is_stored_after_a_newer_version_was_found() {
    let answers = StoreSharingDriver::run();
    assert_eq!(answers["aheadRows"], serde_json::json!(["o1"]));
}
