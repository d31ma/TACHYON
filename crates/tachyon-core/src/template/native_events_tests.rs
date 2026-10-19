use crate::node_driver::NodeDriver;
use std::path::Path;

struct NativeEventsDriver;

impl NativeEventsDriver {
    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
globalThis.location = new URL('tachyon://app/')
globalThis.document = { querySelector: () => ({ content: 'https://api.example.com' }) }
const opened = []
globalThis.EventSource = class { constructor(url) { this.url = String(url); opened.push(this) } addEventListener() {} close() {} }
globalThis.WebSocket = class { constructor(url) { this.url = String(url); this.readyState = 1; this.sent = []; opened.push(this) } send(text) { this.sent.push(text) } close() {} }
;(0, eval)(`${await readFile('./application-events.js', 'utf8')}\nglobalThis.TacApplicationEvents = TacApplicationEvents`)
;(0, eval)(`${await readFile('./tac-native-events.js', 'utf8')}\nglobalThis.TacNativeEvents = TacNativeEvents`)
const calls = []
let refreshed = 0, changed = 0, failed = 0
const owner = { __tachyonStoredChanged: () => { changed += 1 } }
const declared = {
  listeners: [{ key: 'heardOrder', events: ['OrderPlaced'] }, { key: 7, events: ['Ignored'] }, { key: 'heardTick', events: ['Tick'] }],
  streams: [
    { client: 'chat', path: '/api/chat', event: 'Said', socket: true },
    { client: 'ticks', path: '/api/ticks', event: 'Tick' },
    { client: 'elsewhere', path: '//evil.example/api', event: 'Said' },
    { client: 'relative', path: 'api/ticks', event: 'Tick' },
  ],
}
const call = async (payload) => { calls.push(payload) }
const refresh = async () => { refreshed += 1 }
TacNativeEvents.bind(owner, declared, call, refresh, { OrderPlaced: '/api/topics/OrderPlaced', Tick: '/api/topics/Tick' })
const plan = TacNativeEvents.plans.get(owner)
const events = new TacApplicationEvents({ changed: () => {}, failed: () => { failed += 1 } })
events.attach(owner, plan, {})
await events.initialize(owner)
const find = (path) => opened.find((source) => source.url.endsWith(path))
find('/api/topics/OrderPlaced').onmessage({ data: JSON.stringify({ item: 'chair' }) })
find('/api/ticks').onmessage({ data: JSON.stringify({ name: 'ANVIL-1', count: 3 }) })
TacNativeEvents.publish(owner, () => events.publish('OrderPlaced', { item: 'its own echo' }))
await TacNativeEvents.send(owner, { send: 'chat', value: { text: 'Hello' } })
let unknown = 'sent'
try { TacNativeEvents.send(owner, { send: 'missing', value: 1 }) } catch (error) { unknown = error.constructor.name }
await new Promise((resolve) => setTimeout(resolve, 20))
console.log(JSON.stringify({
  listeners: plan.listeners.map(({ key, method }) => ({ key, method })),
  streams: plan.streams.map(({ method, path, socket, topic, sender }) => ({ method, path, socket, topic, sender: sender ?? null })),
  opened: opened.map((source) => source.url),
  sent: find('/api/chat').sent,
  calls, refreshed, changed, failed, unknown,
}))
process.exit(0)
";

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        for (name, source) in [
            (
                "application-events.js",
                include_str!("application-events.js"),
            ),
            ("tac-native-events.js", include_str!("tac-native-events.js")),
            ("drive.mjs", Self::DRIVER),
        ] {
            Self::write(root.path(), name, source);
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

    fn write(root: &Path, name: &str, source: &str) {
        std::fs::write(root.join(name), source).expect("write");
    }
}

#[test]
fn the_page_holds_a_linked_service_streams_sockets_and_topics() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = NativeEventsDriver::run();
    assert_eq!(
        report["listeners"],
        serde_json::json!([
            { "key": "heardOrder", "method": "__tachyonListen:heardOrder" },
            { "key": "heardTick", "method": "__tachyonListen:heardTick" },
        ]),
        "{report}"
    );
    assert_eq!(
        report["streams"],
        serde_json::json!([
            { "method": "__tachyonStream", "path": "/api/chat", "socket": true, "topic": false, "sender": "__tachyonSend:chat" },
            { "method": "__tachyonStream", "path": "/api/ticks", "socket": false, "topic": false, "sender": null },
            { "method": "", "path": "/api/topics/OrderPlaced", "socket": false, "topic": true, "sender": null },
            { "method": "", "path": "/api/topics/Tick", "socket": false, "topic": true, "sender": null },
        ]),
        "{report}"
    );
    assert_eq!(
        report["opened"],
        serde_json::json!([
            "wss://api.example.com/api/chat",
            "https://api.example.com/api/ticks",
            "https://api.example.com/api/topics/OrderPlaced",
            "https://api.example.com/api/topics/Tick",
        ]),
        "{report}"
    );
    assert_eq!(
        report["sent"],
        serde_json::json!([r#"{"text":"Hello"}"#]),
        "{report}"
    );
    assert_eq!(
        report["calls"],
        serde_json::json!([
            { "op": "event", "name": "heardOrder", "value": { "item": "chair" } },
            { "op": "event", "name": "heardTick", "value": { "name": "ANVIL-1", "count": 3 } },
        ]),
        "{report}"
    );
    assert_eq!(report["refreshed"], 2, "{report}");
    assert_eq!(report["changed"], 2, "{report}");
    assert_eq!(report["failed"], 0, "{report}");
    assert_eq!(report["unknown"], "TypeError", "{report}");
}
