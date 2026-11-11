use crate::node_driver::NodeDriver;
use serde_json::{Value, json};

struct StoredFieldLimitDriver;

impl StoredFieldLimitDriver {
    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
const client = await readFile('./tac-client.js', 'utf8')
const host = client.slice(client.indexOf('const webStorage = '), client.indexOf('const persistScope = '))
class MemoryStorage {
  constructor() { this.items = new Map() }
  getItem(key) { return this.items.has(key) ? this.items.get(key) : null }
  setItem(key, value) { this.items.set(key, String(value)) }
  removeItem(key) { this.items.delete(key) }
}
const local = new MemoryStorage()
const session = new MemoryStorage()
const page = new Function('localStorage', 'sessionStorage', `${host}\nreturn { Owners, bindPersistentFields, NeededServices }`)(local, session)
const outcome = (work) => {
  try { work(); return 'saved' } catch (error) { return `${error.name}: ${error.message}` }
}
const owner = { note: 'first', draft: 'first' }
page.Owners.stored.set(owner, { note: 'local', draft: 'session' })
page.bindPersistentFields(owner, 'service:notes:/')
const plain = {
  atLimit: outcome(() => { owner.note = 'a'.repeat(65534) }),
  overLimit: outcome(() => { owner.note = 'b'.repeat(65535) }),
  kept: owner.note.length,
  wideAtLimit: outcome(() => { owner.draft = 'é'.repeat(32767) }),
  wideOverLimit: outcome(() => { owner.draft = 'é'.repeat(32768) }),
  keptWide: owner.draft.length,
}
plain.stored = { note: local.getItem('tac:service:notes:/:note').length, draft: session.getItem('tac:service:notes:/:draft').length }
let refused = ''
try {
  await page.NeededServices.save({ ready: Promise.resolve(), holder: { target: { __tachyonStoredChanged() { throw new RangeError('held') } } } })
} catch (error) { refused = error.message }
const quiet = await page.NeededServices.save({ ready: Promise.reject(new Error('gone')), holder: { target: {} } })
console.log(JSON.stringify({ plain, held: { refused, quiet: quiet === undefined } }))
process.exit(0)
";

    fn run() -> Value {
        let root = tempfile::tempdir().expect("root");
        for (name, source) in [
            ("tac-client.js", include_str!("tac-client.js")),
            ("drive.mjs", Self::DRIVER),
        ] {
            std::fs::write(root.path().join(name), source).expect("write driver file");
        }
        let mut command = NodeDriver::command();
        command.args(["drive.mjs"]).current_dir(root.path());
        let output = NodeDriver::output(&mut command).expect("node drive.mjs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.lines().last().unwrap_or_default()).expect("report")
    }

    fn refused(key: &str, bytes: usize) -> String {
        format!(
            "RangeError: Tac did not save the stored field '{key}': its JSON is {bytes} bytes, and a @LocalState or @SessionState value holds at most 65536 bytes (64 KiB). Keep large data in a Store table."
        )
    }
}

#[test]
fn an_oversized_stored_field_assignment_fails_and_keeps_the_saved_value() {
    let report = StoredFieldLimitDriver::run();
    assert_eq!(
        report["plain"],
        json!({
            "atLimit": "saved",
            "overLimit": StoredFieldLimitDriver::refused("tac:service:notes:/:note", 65537),
            "kept": 65534,
            "wideAtLimit": "saved",
            "wideOverLimit": StoredFieldLimitDriver::refused("tac:service:notes:/:draft", 65538),
            "keptWide": 32767,
            "stored": { "note": 65536, "draft": 32769 },
        })
    );
}

#[test]
fn a_held_service_reports_an_oversized_field_but_not_a_failed_refresh() {
    let report = StoredFieldLimitDriver::run();
    assert_eq!(report["held"], json!({ "refused": "held", "quiet": true }));
}
