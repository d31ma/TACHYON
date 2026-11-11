use crate::node_driver::NodeDriver;
use serde_json::{Value, json};
use std::path::Path;

struct ReconnectDriver;

impl ReconnectDriver {
    const PAGE: &'static str = r#"import { readFile } from 'node:fs/promises'
;(0, eval)(`${await readFile('./application-events.js', 'utf8')}\nglobalThis.TacApplicationEvents = TacApplicationEvents`)
const timers = []
globalThis.setTimeout = (work, delay) => { timers.push({ work, delay }); return timers.length }
globalThis.clearTimeout = (id) => { if (timers[id - 1]) timers[id - 1].cleared = true }
const due = () => timers.filter((timer) => !timer.cleared && timer.delay !== 60000)
const fire = () => { const [next] = due(); next.cleared = true; next.work(); return next.delay }
const heard = { online: [], visibilitychange: [] }
const metas = {}
globalThis.addEventListener = (name, listener) => heard[name]?.push(listener)
globalThis.location = new URL('https://app.example.com/page')
globalThis.document = { visibilityState: 'visible', addEventListener: (name, listener) => heard[name]?.push(listener),
  querySelector: (selector) => { const name = /name="([^"]+)"/.exec(selector)?.[1]; return metas[name] ? { content: metas[name] } : null } }
Object.defineProperty(globalThis, 'navigator', { configurable: true, writable: true, value: { onLine: true } })
const sockets = [], sources = []
globalThis.WebSocket = class { constructor(url) { this.url = String(url); this.readyState = 0; sockets.push(this) } send() {} close() { this.readyState = 3 } }
globalThis.EventSource = class { constructor(url) { this.url = String(url); sources.push(this) } addEventListener() {} close() {} }
let failures = 0
const hold = async (path, socket = true) => {
  const owner = { opened: [], closed: [], receive() {}, heardOpen(value) { this.opened.push(value) }, heardClose(value) { this.closed.push(value) } }
  const bus = new TacApplicationEvents({ changed: () => {}, failed: () => { failures += 1 } })
  bus.attach(owner, { listeners: [{ method: 'heardOpen', event: 'SocketOpened' }, { method: 'heardClose', event: 'SocketClosed' }],
    streams: [{ method: 'receive', path, event: 'Said', socket }] }, {})
  await bus.initialize(owner)
  return { owner, done: () => bus.dispose(owner) }
}
const drop = (code = 1006) => sockets.at(-1).onclose({ code })
const report = {}
"#;

    const STEPS: &'static str = r"metas['tachyon-yon-origins'] = JSON.stringify({ realtime: 'https://rt.example.com' })
let held = await hold('/.tachyon/reconnect/0/100/400/50/.tachyon/origin/realtime/chat')
report.unlimited = { url: sockets.at(-1).url, delays: [] }
for (let attempt = 0; attempt < 8; attempt += 1) { drop(); report.unlimited.delays.push(fire()) }
report.unlimited.sockets = sockets.length
report.unlimited.reconnecting = held.owner.closed.map((closed) => closed.reconnecting)
report.unlimited.paths = [...new Set(held.owner.closed.map((closed) => closed.path))]
sockets.at(-1).readyState = 1
sockets.at(-1).onopen()
report.unlimited.opened = held.owner.opened
drop()
report.unlimited.afterOpen = fire()
held.done()

held = await hold('/chat')
report.limited = { url: sockets.at(-1).url, delays: [] }
for (let attempt = 0; attempt < 5; attempt += 1) { drop(); report.limited.delays.push(fire()) }
const before = sockets.length
drop()
report.limited.waiting = due().length
report.limited.lastReconnecting = held.owner.closed.at(-1).reconnecting
for (const listener of heard.visibilitychange) listener()
report.limited.revived = sockets.length - before
held.done()

held = await hold('/.tachyon/reconnect/3/100/400/0/chat')
navigator.onLine = false
drop()
report.offline = { waiting: due().length, reconnecting: held.owner.closed.at(-1).reconnecting }
const offlineCount = sockets.length
navigator.onLine = true
for (const listener of heard.online) listener()
report.offline.reopened = sockets.length - offlineCount
report.offline.waitingAfter = due().length
held.done()

held = await hold('/chat')
drop(1000)
const finalCount = sockets.length
for (const listener of heard.visibilitychange) listener()
report.final = { waiting: due().length, reopened: sockets.length - finalCount, reconnecting: held.owner.closed.at(-1).reconnecting }
held.done()

const failedBefore = failures, socketsBefore = sockets.length
held = await hold('/.tachyon/origin/missing/chat')
report.unknown = { failures: failures - failedBefore, sockets: sockets.length - socketsBefore }
held.done()

held = await hold('/.tachyon/origin/realtime/feed', false)
report.stream = sources.at(-1).url
held.done()
console.log(JSON.stringify(report))
process.exit(0)
";

    const HTTP: &'static str = r#"import { readFile } from 'node:fs/promises'
globalThis.location = new URL('https://app.example.com/page')
const metas = { 'tachyon-yon-origins': JSON.stringify({ realtime: 'https://rt.example.com' }) }
globalThis.document = { querySelector: (selector) => { const name = /name="([^"]+)"/.exec(selector)?.[1]; return metas[name] ? { content: metas[name] } : null } }
const fetched = []
globalThis.fetch = async (url) => { fetched.push(String(url)); return new Response('{"ok":true}', { headers: { 'content-type': 'application/json' } }) }
;(0, eval)(`${await readFile('./tac-http.js', 'utf8')}\nglobalThis.TacBrowserHttp = TacBrowserHttp`)
;(0, eval)(`${await readFile('./client_runtime.js', 'utf8')}\nglobalThis.exchange = __tachyon_client_exchange__`)
const http = globalThis[Symbol.for('tachyon.tac.http')]
const ask = (path) => http.start(JSON.stringify({ method: 'GET', path, redirect: 'error', credentials: 'omit', deadline: Date.now() + 5000, maxResponseBytes: 1000 })).answer
const report = { named: JSON.parse(await ask('/.tachyon/origin/realtime/messages')), missing: JSON.parse(await ask('/.tachyon/origin/missing/messages')) }
report.page = JSON.parse(await ask('/messages'))
report.script = await exchange({ method: 'GET', path: '/.tachyon/origin/realtime/rooms', query: {}, headers: {}, maxResponseBytes: 1000 })
report.scriptMissing = await exchange({ method: 'GET', path: '/.tachyon/origin/missing/rooms', query: {}, headers: {}, maxResponseBytes: 1000 }).then(() => 'sent', (error) => error.constructor.name)
report.fetched = fetched
console.log(JSON.stringify(report))
process.exit(0)
"#;

    fn run(driver: &str) -> Value {
        let root = tempfile::tempdir().expect("root");
        for (name, source) in [
            (
                "application-events.js",
                include_str!("application-events.js"),
            ),
            ("tac-http.js", include_str!("tac-http.js")),
            (
                "client_runtime.js",
                include_str!("../behavior/client_runtime.js"),
            ),
            ("drive.mjs", driver),
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

    fn bus() -> Value {
        Self::run(&format!("{}{}", Self::PAGE, Self::STEPS))
    }

    fn write(root: &Path, name: &str, source: &str) {
        std::fs::write(root.join(name), source).expect("write");
    }

    fn delay(report: &Value, index: usize) -> u64 {
        report["unlimited"]["delays"][index]
            .as_u64()
            .expect("delay")
    }
}

#[test]
fn a_socket_with_no_try_limit_keeps_reconnecting_to_its_named_origin_with_a_capped_delay() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = ReconnectDriver::bus();
    assert_eq!(report["unlimited"]["url"], "wss://rt.example.com/chat");
    assert_eq!(report["unlimited"]["sockets"], 9);
    assert_eq!(
        report["unlimited"]["reconnecting"],
        json!([true, true, true, true, true, true, true, true])
    );
    assert_eq!(report["unlimited"]["paths"], json!(["/chat"]));
    for (index, base) in [100, 200, 400, 400, 400, 400, 400, 400]
        .into_iter()
        .enumerate()
    {
        let delay = ReconnectDriver::delay(&report, index);
        assert!((base..=base + 50).contains(&delay), "{index}: {delay}");
    }
    assert_eq!(
        report["unlimited"]["opened"],
        json!([{ "path": "/chat", "reconnected": false }])
    );
    let after_open = report["unlimited"]["afterOpen"].as_u64().expect("delay");
    assert!(
        (100..=150).contains(&after_open),
        "tries restart at open: {after_open}"
    );
}

#[test]
fn the_default_policy_stops_after_five_tries_and_a_shown_page_reconnects_at_once() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = ReconnectDriver::bus();
    assert_eq!(report["limited"]["url"], "wss://app.example.com/chat");
    assert_eq!(
        report["limited"]["delays"],
        json!([1000, 2000, 4000, 8000, 16000])
    );
    assert_eq!(report["limited"]["waiting"], 0);
    assert_eq!(report["limited"]["lastReconnecting"], false);
    assert_eq!(report["limited"]["revived"], 1);
}

#[test]
fn an_offline_page_waits_without_counting_and_reconnects_when_it_is_online() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = ReconnectDriver::bus();
    assert_eq!(
        report["offline"],
        json!({ "waiting": 0, "reconnecting": true, "reopened": 1, "waitingAfter": 0 })
    );
    assert_eq!(
        report["final"],
        json!({ "waiting": 0, "reopened": 0, "reconnecting": false })
    );
}

#[test]
fn an_undeclared_named_origin_fails_closed_and_a_stream_reaches_its_named_origin() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = ReconnectDriver::bus();
    assert_eq!(report["unknown"], json!({ "failures": 1, "sockets": 0 }));
    assert_eq!(report["stream"], "https://rt.example.com/feed");
}

#[test]
fn http_requests_from_every_language_reach_a_declared_named_origin_only() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = ReconnectDriver::run(ReconnectDriver::HTTP);
    assert_eq!(report["named"], json!({ "value": { "ok": true } }));
    assert_eq!(
        report["missing"],
        json!({ "error": { "code": "HTTP_ORIGIN" } })
    );
    assert_eq!(report["page"], json!({ "value": { "ok": true } }));
    assert_eq!(report["script"], json!({ "ok": true }));
    assert_eq!(report["scriptMissing"], "TypeError");
    assert_eq!(
        report["fetched"],
        json!([
            "https://rt.example.com/messages",
            "https://app.example.com/messages",
            "https://rt.example.com/rooms"
        ])
    );
}
