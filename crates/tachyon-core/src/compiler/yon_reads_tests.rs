#![allow(clippy::expect_used)]

use super::yon_reads::YonReads;
use crate::node_driver::NodeDriver;
use std::path::Path;

struct YonReadsDriver;

impl YonReadsDriver {
    const RULES: &'static str = r#"[{"pattern":"^/api/[^?]*$","policy":"network-first"},{"pattern":"^/cached/[^?]*$","policy":"cache-first"},{"pattern":"^/fresh$","policy":"no-store"}]"#;

    const STUBS: &'static str = r#"import { readFile } from 'node:fs/promises'
globalThis.location = new URL('https://appassets.tachyon.local/notes')
const declared = { content: 'https://api.example.com' }
globalThis.document = { querySelector: (selector) => selector === 'meta[name="tachyon-yon-origin"]' ? declared : null }
class MemoryCache {
  entries = new Map()
  async match(request) { return this.entries.get(String(request.url ?? request))?.clone() }
  async put(request, response) { this.entries.set(String(request.url), response) }
  async delete(request) { return this.entries.delete(String(request.url ?? request)) }
  async keys() { return [...this.entries.keys()].map((url) => new Request(url)) }
}
const store = new MemoryCache()
globalThis.caches = { open: async () => store }
const network = { online: true, calls: [], count: 0 }
const answers = {
  '/api/notes': () => ({ 'cache-control': 'public, max-age=60', vary: 'Origin, Accept-Encoding' }),
  '/api/list': () => ({ 'cache-control': 'public, max-age=60' }),
  '/api/secret': () => ({ 'cache-control': 'no-store' }),
  '/api/varied': () => ({ 'cache-control': 'public', vary: 'Accept' }),
  '/api/local': () => ({ 'cache-control': 'public' }),
  '/cached/item': () => ({ 'cache-control': 'public' }),
  '/fresh': () => ({ 'cache-control': 'public' }),
}
globalThis.fetch = async (input, init) => {
  const request = new Request(input, init)
  network.calls.push(`${request.method} ${new URL(request.url).pathname} ${request.cache}`)
  if (!network.online) throw new TypeError('offline')
  network.count += 1
  const headers = answers[new URL(request.url).pathname]?.() ?? {}
  return new Response(JSON.stringify({ text: `answer ${network.count}` }), { headers: { 'content-type': 'application/json', ...headers } })
}
const outcome = async (work) => { try { return await work() } catch (error) { return `refused: ${error.message}` } }
"#;

    const DRIVER: &'static str = r"
;(0, eval)(`${await readFile('./client_runtime.js', 'utf8')}\nglobalThis.exchange = __tachyon_client_exchange__`)
;(0, eval)(await readFile('./tac-http.js', 'utf8'))
;(0, eval)(await readFile('./tac-yon-reads.js', 'utf8'))
const read = (path, extra = {}) => outcome(() => exchange({ path, method: 'GET', query: {}, headers: {}, publicRead: true, maxResponseBytes: 1024, ...extra }))
const http = globalThis[Symbol.for('tachyon.tac.http')]
const ticket = async (path) => JSON.parse(await http.start(JSON.stringify({ method: 'GET', path, redirect: 'error', credentials: 'omit', deadline: Date.now() + 5000, maxResponseBytes: 1024, publicRead: true })).answer)
const offline = async (work) => { network.online = false; try { return await work() } finally { network.online = true } }
const report = {}
report.networkFirst = [await read('/api/notes'), await offline(() => read('/api/notes'))]
report.cacheFirst = [await read('/cached/item'), await read('/cached/item'), await read('/cached/item', { revalidate: true })]
report.serverNoStore = [await read('/api/secret'), await offline(() => read('/api/secret'))]
report.uncachedRead = [await read('/api/list', { publicRead: false }), await offline(() => read('/api/list', { publicRead: false }))]
report.services = [await ticket('/api/list'), await offline(() => ticket('/api/list'))]
report.ruleNoStore = [await read('/fresh'), await offline(() => read('/fresh'))]
report.varied = [await read('/api/varied'), await offline(() => read('/api/varied'))]
report.pageOrigin = [await read(`${location.origin}/api/local`), await offline(() => read(`${location.origin}/api/local`))]
await outcome(() => exchange({ path: '/api/notes', method: 'POST', query: {}, headers: {}, body: { text: 'new' }, maxResponseBytes: 1024 }))
report.afterWrite = await offline(() => read('/api/notes'))
report.kept = [...store.entries.keys()].sort()
report.calls = network.calls
console.log(JSON.stringify(report))
process.exit(0)
";

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        let reads = String::from_utf8(YonReads::script(Self::RULES)).expect("utf8");
        for (name, source) in [
            (
                "client_runtime.js",
                include_str!("../behavior/client_runtime.js"),
            ),
            ("tac-http.js", include_str!("../template/tac-http.js")),
            ("tac-yon-reads.js", reads.as_str()),
        ] {
            Self::write(root.path(), name, source);
        }
        Self::write(
            root.path(),
            "drive.mjs",
            &format!("{}{}", Self::STUBS, Self::DRIVER),
        );
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

    fn write(root: &Path, name: &str, source: &str) {
        std::fs::write(root.join(name), source).expect("write");
    }
}

#[test]
fn a_native_page_applies_the_cache_rules_to_yon_reads() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = YonReadsDriver::run();
    let text = |value: &serde_json::Value| value["text"].as_str().map(String::from);
    assert_eq!(
        report["networkFirst"]
            .as_array()
            .expect("reads")
            .iter()
            .map(text)
            .collect::<Vec<_>>(),
        [
            Some(String::from("answer 1")),
            Some(String::from("answer 1"))
        ],
        "{report}"
    );
    assert_eq!(report["cacheFirst"][0]["text"], "answer 2", "{report}");
    assert_eq!(report["cacheFirst"][1]["text"], "answer 2", "{report}");
    assert_eq!(report["cacheFirst"][2]["text"], "answer 3", "{report}");
    for refused in [
        "serverNoStore",
        "uncachedRead",
        "ruleNoStore",
        "varied",
        "pageOrigin",
    ] {
        assert!(
            report[refused][0]["text"].is_string(),
            "{refused}: {report}"
        );
        assert_eq!(
            report[refused][1], "refused: offline",
            "{refused}: {report}"
        );
    }
    assert_eq!(
        report["services"][0]["value"]["text"], report["services"][1]["value"]["text"],
        "{report}"
    );
    assert_eq!(report["afterWrite"], "refused: offline", "{report}");
    assert_eq!(
        report["kept"],
        serde_json::json!([
            "https://api.example.com/api/list",
            "https://api.example.com/cached/item"
        ]),
        "{report}"
    );
    let calls = report["calls"].as_array().expect("calls");
    assert!(
        calls.contains(&serde_json::json!("GET /api/notes no-store")),
        "a network-first read bypasses the HTTP cache: {report}"
    );
    assert!(
        calls.contains(&serde_json::json!("GET /api/list no-store")),
        "{report}"
    );
    assert_eq!(
        calls
            .iter()
            .filter(|call| call
                .as_str()
                .is_some_and(|call| call.contains("/cached/item")))
            .count(),
        2,
        "the second cache-first read is answered from the cache: {report}"
    );
}

#[test]
fn only_a_native_build_with_a_yon_origin_and_rules_ships_the_read_cache() {
    let root = tempfile::tempdir().expect("project");
    let pages = root.path().join("client/pages");
    std::fs::create_dir_all(&pages).expect("pages");
    std::fs::write(pages.join("tac.html"), "<page-home />").expect("page");
    let component = root.path().join("client/components/page/home");
    std::fs::create_dir_all(&component).expect("component");
    std::fs::write(component.join("tac.html"), "<main>Home</main>").expect("component view");
    let manifest = |extra: &str| {
        std::fs::write(
            root.path().join("client/manifest.json"),
            format!(
                r#"{{"tac":{{"cache":[{{"path":"/api/*","policy":"network-first"}}]{extra}}}}}"#
            ),
        )
        .expect("manifest");
    };
    let built = |native: bool| {
        let output = tempfile::tempdir().expect("output");
        let project = crate::ProjectDiscovery::discover(root.path()).expect("project");
        let options = super::BuildOptions {
            output_directory: output.path().to_path_buf(),
            incremental: false,
            ..super::BuildOptions::default()
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let target = native.then_some(tachyon_contracts::NativeTarget::Android);
        runtime
            .block_on(super::WebCompiler::build_project_with_target(
                &project, &options, target,
            ))
            .expect("build");
        let page = std::fs::read_to_string(output.path().join("index.html")).expect("page");
        let script = std::fs::read_to_string(output.path().join(YonReads::PATH)).ok();
        (page, script)
    };
    manifest(r#","yon":{"origin":"https://api.example.com"}"#);
    let (page, script) = built(true);
    let tag =
        r#"<script type="module" src="/.tachyon/tac-yon-reads.js" data-tachyon-runtime></script>"#;
    let reads = page
        .find(tag)
        .expect("the native page loads the read cache");
    let runtime = page.find("/.tachyon/tac-client.js").expect("runtime");
    assert!(
        reads < runtime,
        "the read cache loads before the runtime: {page}"
    );
    let script = script.expect("the native build ships the read cache");
    assert!(
        script.contains(r#"{"pattern":"^/api/[^?]*$","policy":"network-first"}"#),
        "{script}"
    );
    assert!(!script.contains("__CACHE_RULES__"), "{script}");
    let (page, script) = built(false);
    assert!(
        !page.contains("tac-yon-reads"),
        "a web build keeps the service worker: {page}"
    );
    assert!(script.is_none());
    manifest("");
    let (page, script) = built(true);
    assert!(
        !page.contains("tac-yon-reads"),
        "no Yon origin, no Yon reads: {page}"
    );
    assert!(script.is_none());
}
