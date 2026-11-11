#![allow(clippy::expect_used)]

use crate::node_driver::NodeDriver;
use serde_json::json;

struct NativeHeldPage;

impl NativeHeldPage {
    const DRIVER: &'static str = r"
import { readFile } from 'node:fs/promises'
globalThis.location = { pathname: '/' }
class MemoryStorage {
  constructor() { this.items = new Map() }
  getItem(key) { return this.items.has(key) ? this.items.get(key) : null }
  setItem(key, value) { this.items.set(key, String(value)) }
  removeItem(key) { this.items.delete(key) }
}
const fresh = () => ({ visits: 0, draft: '' })
const host = { counter: fresh(), owners: new Map(), requests: [] }
const answer = (value) => JSON.stringify({ value })
globalThis.__tachyonNativeHostCall = (capability, payload) => {
  const request = JSON.parse(payload)
  host.requests.push(request)
  if (request.op === 'init' && request.held === undefined) {
    host.owners.set(request.owner, 'root')
    return answer({ fields: [], methods: ['visit'], listeners: [], streams: [], needs: ['counter'] })
  }
  if (request.op === 'init') {
    host.owners.set(request.owner, 'held')
    return answer({ fields: ['visits', 'draft'], methods: [], listeners: [], streams: [], needs: ['counter'] })
  }
  if (request.op === 'dispose') { host.owners.delete(request.owner); return answer(null) }
  if (request.op === 'call') { host.counter.visits += 1; return answer(null) }
  if (request.op === 'get') return answer(host.counter[request.name])
  host.counter[request.name] = request.value
  return answer(null)
}
const client = await readFile('./tac-client.js', 'utf8')
const between = (start, end) => client.slice(client.indexOf(start), client.indexOf(end))
const source = [
  between('class NativeState {', 'class NativeTasks {'),
  between('const webStorage = ', 'const persistScope = '),
  between('const nativeHost = ', 'if (nativeHost()) {'),
].join('\n')
const local = new MemoryStorage()
const tab = new MemoryStorage()
local.setItem('tac:service:counter:/:visits', '5')
tab.setItem('tac:service:counter:/:draft', '\x22kept draft\x22')
const stubs = 'const ClientView = { plan: { route: \x22/\x22 } }\n' +
  'const NativeTasks = { settle: (_, value) => value, cancel() {} }\n' +
  'const TacNativeEvents = { bind() {} }\n' +
  'const Services = { catalog: {} }\n' +
  'const markDocument = () => {}\n' +
  'const signalFailure = () => {}\n'
const page = new Function('localStorage', 'sessionStorage', `${stubs}${source}\nreturn { bindNativeInstance, bindPersistentFields, disposeNativeInstance, NeededServices, Owners }`)(local, tab)
const saved = { counter: { visits: 'local', draft: 'session' } }
const settle = () => new Promise((resolve) => setTimeout(resolve, 20))
const start = async () => {
  const owner = {}
  await page.bindNativeInstance(owner, 'writer', {})
  page.Owners.stored.set(owner, {})
  page.bindPersistentFields(owner, 'service:writer:/')
  await page.NeededServices.bind(owner, owner.__tachyonNeeds, saved, page.NeededServices.holder(owner))
  await settle()
  return owner
}
const close = async (owner) => {
  page.disposeNativeInstance(owner)
  page.NeededServices.release(owner)
  await settle()
}
const first = await start()
const needs = [...first.__tachyonNeeds]
const opened = host.requests.filter((request) => request.held !== undefined).map(({ service, held }) => ({ service, held }))
const restored = { ...host.counter }
await first.visit()
await settle()
const written = { local: Object.fromEntries(local.items), session: Object.fromEntries(tab.items) }
await close(first)
const open = host.owners.size
host.counter = fresh()
const second = await start()
const reloaded = { ...host.counter }
await close(second)
console.log(JSON.stringify({ needs, opened, restored, written, open, reloaded }))
process.exit(0)
";

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("page");
        let client = include_str!("../template/tac-client.js");
        std::fs::write(root.path().join("tac-client.js"), client).expect("client");
        std::fs::write(root.path().join("drive.mjs"), Self::DRIVER).expect("driver");
        let output = NodeDriver::output(
            NodeDriver::command()
                .arg("drive.mjs")
                .current_dir(root.path()),
        )
        .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let last = stdout.lines().last().unwrap_or_default();
        serde_json::from_str(last).unwrap_or_else(|error| panic!("{error}: {stdout}"))
    }
}

#[test]
fn a_native_rust_held_service_keeps_its_saved_fields_on_the_page() {
    let report = NativeHeldPage::run();
    assert_eq!(report["needs"], json!(["counter"]), "{report}");
    assert_eq!(
        report["opened"],
        json!([{"service": "writer", "held": "counter"}]),
        "the holder goes through the service that holds it: {report}"
    );
    assert_eq!(
        report["restored"],
        json!({"visits": 5, "draft": "kept draft"}),
        "{report}"
    );
    assert_eq!(
        report["written"],
        json!({
            "local": {"tac:service:counter:/:visits": "6"},
            "session": {"tac:service:counter:/:draft": "\"kept draft\""},
        }),
        "{report}"
    );
    assert_eq!(
        report["open"],
        json!(0),
        "releasing the owner closes its holder: {report}"
    );
    assert_eq!(
        report["reloaded"],
        json!({"visits": 6, "draft": "kept draft"}),
        "{report}"
    );
}
