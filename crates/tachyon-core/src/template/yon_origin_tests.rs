use crate::node_driver::NodeDriver;
use std::path::Path;

struct YonOriginDriver;

impl YonOriginDriver {
    const STUBS: &'static str = r#"import { readFile } from 'node:fs/promises'
const declared = { content: process.argv[2] }
globalThis.location = new URL(process.argv[3])
globalThis.document = { querySelector: (selector) => selector === 'meta[name="tachyon-yon-origin"]' && declared.content ? declared : null }
const fetched = []
globalThis.fetch = async (url, init) => {
  fetched.push({ url: String(url), mode: init.mode ?? 'cors', credentials: init.credentials })
  return new Response('{"ok":true}', { headers: { 'content-type': 'application/json' } })
}
const opened = []
globalThis.EventSource = class { constructor(url) { this.url = String(url); opened.push(this.url) } addEventListener() {} close() {} }
globalThis.WebSocket = class { constructor(url) { this.url = String(url); opened.push(this.url) } send() {} close() {} }
const outcome = async (work) => { try { return await work() } catch (error) { return `refused: ${error.message}` } }
"#;

    const DRIVER: &'static str = r"
;(0, eval)(`${await readFile('./client_runtime.js', 'utf8')}\nglobalThis.exchange = __tachyon_client_exchange__`)
;(0, eval)(await readFile('./tac-http.js', 'utf8'))
;(0, eval)(`${await readFile('./application-events.js', 'utf8')}\nglobalThis.TacApplicationEvents = TacApplicationEvents`)
const request = (path) => ({ path, method: 'GET', query: {}, headers: {}, maxResponseBytes: 1024 })
const http = globalThis[Symbol.for('tachyon.tac.http')]
const ticket = async (path) => JSON.parse(await http.start(JSON.stringify({ method: 'GET', path, redirect: 'error', credentials: 'omit', deadline: Date.now() + 5000, maxResponseBytes: 1024 })).answer)
const client = {
  relative: await outcome(() => exchange(request('/api/notes'))),
  yonAbsolute: await outcome(() => exchange(request('https://api.example.com/api/notes'))),
  pageAbsolute: await outcome(() => exchange(request(`${location.origin}/api/notes`))),
  elsewhere: await outcome(() => exchange(request('https://evil.example/api/notes'))),
}
const services = { relative: await ticket('/api/notes') }
const events = new TacApplicationEvents({ changed: () => {}, failed: () => {} })
const state = () => ({ disposed: false, controller: new AbortController() })
events.stream({}, state(), { path: '/api/ticks', event: 'Tick' })
events.stream({}, state(), { path: '/api/chat', event: 'Said', socket: true })
console.log(JSON.stringify({ client, services, fetched, opened }))
process.exit(0)
";

    fn run(declared: &str, page: &str) -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        for (name, source) in [
            (
                "client_runtime.js",
                include_str!("../behavior/client_runtime.js"),
            ),
            ("tac-http.js", include_str!("tac-http.js")),
            (
                "application-events.js",
                include_str!("application-events.js"),
            ),
        ] {
            Self::write(root.path(), name, source);
        }
        Self::write(
            root.path(),
            "drive.mjs",
            &format!("{}{}", Self::STUBS, Self::DRIVER),
        );
        let mut command = NodeDriver::command();
        command
            .args(["drive.mjs", declared, page])
            .current_dir(root.path());
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
fn a_native_page_sends_every_yon_call_to_the_declared_origin() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = YonOriginDriver::run("https://api.example.com", "https://tachyon.local/notes");
    assert_eq!(report["client"]["relative"]["ok"], true, "{report}");
    assert_eq!(report["client"]["yonAbsolute"]["ok"], true, "{report}");
    assert_eq!(report["client"]["pageAbsolute"]["ok"], true, "{report}");
    assert!(
        report["client"]["elsewhere"]
            .as_str()
            .is_some_and(|text| text.starts_with("refused: Tac Client requires")),
        "{report}"
    );
    assert_eq!(
        report["services"]["relative"]["value"]["ok"], true,
        "{report}"
    );
    assert_eq!(
        report["fetched"],
        serde_json::json!([
            { "url": "https://api.example.com/api/notes", "mode": "cors", "credentials": "omit" },
            { "url": "https://api.example.com/api/notes", "mode": "cors", "credentials": "omit" },
            { "url": "https://tachyon.local/api/notes", "mode": "cors", "credentials": "omit" },
            { "url": "https://api.example.com/api/notes", "mode": "cors", "credentials": "omit" },
        ]),
        "{report}"
    );
    assert_eq!(
        report["opened"],
        serde_json::json!([
            "https://api.example.com/api/ticks",
            "wss://api.example.com/api/chat"
        ]),
        "{report}"
    );
}

#[test]
fn without_a_declared_origin_yon_calls_stay_on_the_page_origin() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = YonOriginDriver::run("", "http://127.0.0.1:4000/notes");
    assert_eq!(report["client"]["relative"]["ok"], true, "{report}");
    assert!(
        report["client"]["yonAbsolute"]
            .as_str()
            .is_some_and(|text| text.starts_with("refused:")),
        "{report}"
    );
    assert_eq!(
        report["services"]["relative"]["value"]["ok"], true,
        "{report}"
    );
    assert_eq!(
        report["fetched"][0]["url"], "http://127.0.0.1:4000/api/notes",
        "{report}"
    );
    assert_eq!(report["fetched"][2]["mode"], "same-origin", "{report}");
    assert_eq!(
        report["opened"],
        serde_json::json!(["/api/ticks", "ws://127.0.0.1:4000/api/chat"]),
        "{report}"
    );
}
