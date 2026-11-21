#![allow(clippy::expect_used)]

use super::page_module::PageModule;
use crate::node_driver::NodeDriver;
use std::path::Path;

struct LocksDriver;

impl LocksDriver {
    const WEB_RUNTIME: &'static str = include_str!("../template/tac-web.js");

    const DRIVER: &'static str = r#"import { readFile } from 'node:fs/promises'
const grants = { content: 'locks' }
globalThis.document = { visibilityState: 'visible', addEventListener: () => {}, querySelector: (selector) => selector === 'meta[name="tachyon-browser-apis"]' ? grants : null }
class FakeLocks {
  held = new Set()
  queue = []
  asked = []
  request(name, options, callback) {
    this.asked.push(name)
    return new Promise((resolve, reject) => {
      const start = () => { this.held.add(name); Promise.resolve(callback()).then((value) => { this.held.delete(name); resolve(value); this.next(name) }) }
      if (!this.held.has(name)) { start(); return }
      const waiting = { name, start }
      this.queue.push(waiting)
      options.signal.addEventListener('abort', () => { this.queue = this.queue.filter((item) => item !== waiting); reject(new DOMException('stopped', 'AbortError')) })
    })
  }
  next(name) {
    const index = this.queue.findIndex((item) => item.name === name)
    if (index >= 0) this.queue.splice(index, 1)[0].start()
  }
}
const locks = new FakeLocks()
Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { onLine: true, language: 'en', locks } })
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const web = globalThis[Symbol.for('tachyon.tac.web')]
const sent = []
TacWebApis.events = { publish: () => {}, send: (owner, name, value) => sent.push([owner, name, value]) }
const ask = (owner, call, args) => JSON.parse(web.calling(owner, () => web.host(JSON.stringify({ api: 'locks', call, args }))))
const settle = async () => { for (let round = 0; round < 10; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const take = () => sent.splice(0)
const report = {}
report.noModule = ask('early', 'request', ['queue'])
await settle()
report.noModuleSent = take()
;(0, eval)(await readFile('./tac-locks.js', 'utf8'))
report.first = ask('sender', 'request', ['queue'])
await settle()
report.firstSent = take()
report.held = [...locks.held]
ask('again', 'request', ['queue'])
await settle()
report.againSent = take()
locks.held.add('tachyon:other')
ask('waiter', 'request', ['other'])
await settle()
report.waitingSent = take()
locks.held.delete('tachyon:other')
locks.next('tachyon:other')
await settle()
report.grantedLaterSent = take()
locks.held.add('tachyon:busy')
ask('quitter', 'request', ['busy'])
await settle()
report.cancelled = ask('quitter', 'release', ['busy'])
await settle()
report.cancelledSent = take()
report.released = ask('sender', 'release', ['queue'])
await settle()
report.afterRelease = [...locks.held].sort()
report.unknownRelease = ask('sender', 'release', ['never'])
report.bad = [ask('bad', 'request', ['']), ask('bad', 'request', [7]), ask('bad', 'release', ['x'.repeat(257)])]
await settle()
report.badSent = take()
grants.content = 'share'
report.notGranted = [ask('closed', 'request', ['queue']), ask('closed', 'release', ['queue'])]
grants.content = 'locks'
Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { onLine: true, language: 'en' } })
ask('apple', 'request', ['queue'])
ask('apple-again', 'request', ['queue'])
await settle()
report.appleSent = take()
report.asked = locks.asked
console.log(JSON.stringify(report))
"#;

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        Self::write(root.path(), "tac-web.js", Self::WEB_RUNTIME.as_bytes());
        Self::write(root.path(), "tac-locks.js", &PageModule::LOCKS.script());
        Self::write(root.path(), "drive.mjs", Self::DRIVER.as_bytes());
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

    fn write(root: &Path, name: &str, bytes: &[u8]) {
        std::fs::write(root.join(name), bytes).expect("write");
    }

    fn granted(owner: &str, name: &str) -> serde_json::Value {
        serde_json::json!([owner, "LockGranted", {"name": name}])
    }
}

#[test]
fn the_page_runtime_grants_a_named_lock_and_releases_it() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = LocksDriver::run();
    let started = serde_json::json!({"value": null});
    assert_eq!(report["first"], started, "{report}");
    assert_eq!(
        report["firstSent"],
        serde_json::json!([LocksDriver::granted("sender", "queue")])
    );
    assert_eq!(report["held"], serde_json::json!(["tachyon:queue"]));
    assert_eq!(
        report["againSent"],
        serde_json::json!([LocksDriver::granted("again", "queue")]),
        "a second request for a lock this page holds answers at once: {report}"
    );
    assert_eq!(report["waitingSent"], serde_json::json!([]));
    assert_eq!(
        report["grantedLaterSent"],
        serde_json::json!([LocksDriver::granted("waiter", "other")]),
        "the lock comes when the other tab frees it: {report}"
    );
    assert_eq!(report["released"], started);
    assert_eq!(
        report["afterRelease"],
        serde_json::json!(["tachyon:busy", "tachyon:other"])
    );
    assert_eq!(report["unknownRelease"], started);
    assert_eq!(
        report["asked"],
        serde_json::json!(["tachyon:queue", "tachyon:other", "tachyon:busy"]),
        "one browser request per held name, under the tachyon: prefix: {report}"
    );
}

#[test]
fn the_page_runtime_sends_nothing_for_a_lock_released_while_it_waits() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = LocksDriver::run();
    assert_eq!(report["cancelled"], serde_json::json!({"value": null}));
    assert_eq!(report["cancelledSent"], serde_json::json!([]), "{report}");
}

#[test]
fn the_page_runtime_refuses_bad_names_a_missing_module_and_a_missing_grant() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = LocksDriver::run();
    let name = serde_json::json!({"error": "A lock name is text of 1 to 256 characters."});
    assert_eq!(
        report["bad"],
        serde_json::json!([name, name, name]),
        "{report}"
    );
    assert_eq!(report["badSent"], serde_json::json!([]));
    assert_eq!(
        report["noModule"],
        serde_json::json!({"error": "The locks browser API did not load on this page."})
    );
    assert_eq!(report["noModuleSent"], serde_json::json!([]));
    let not_granted = "The locks browser API is not granted. Add \"locks\" to tac.browserApis in client/manifest.json.";
    assert_eq!(
        report["notGranted"],
        serde_json::json!([{"error": not_granted}, {"error": not_granted}])
    );
}

#[test]
fn the_page_runtime_grants_every_lock_at_once_without_web_locks() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = LocksDriver::run();
    assert_eq!(
        report["appleSent"],
        serde_json::json!([
            LocksDriver::granted("apple", "queue"),
            LocksDriver::granted("apple-again", "queue"),
        ]),
        "the Apple app scheme has no navigator.locks; one window holds every lock: {report}"
    );
}

#[test]
fn only_a_build_that_grants_locks_ships_the_locks_module() {
    let root = tempfile::tempdir().expect("project");
    let pages = root.path().join("client/pages");
    std::fs::create_dir_all(&pages).expect("pages");
    std::fs::write(pages.join("tac.html"), "<page-home />").expect("page");
    let component = root.path().join("client/components/page/home");
    std::fs::create_dir_all(&component).expect("component");
    std::fs::write(component.join("tac.html"), "<main>Home</main>").expect("component view");
    let manifest = |grants: &str, origin: &str| {
        std::fs::write(
            root.path().join("client/manifest.json"),
            format!(r#"{{"tac":{{"origin":"{origin}","browserApis":[{grants}]}}}}"#),
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
        let result = runtime
            .block_on(super::WebCompiler::build_project_with_target(
                &project, &options, target,
            ))
            .expect("build");
        let noted = result
            .service_notes()
            .iter()
            .any(|note| note.starts_with("Locks need a secure page"));
        let page = std::fs::read_to_string(output.path().join("index.html")).expect("page");
        let worker = std::fs::read_to_string(output.path().join("tachyon-sw.js")).expect("sw");
        let shipped = output.path().join(PageModule::LOCKS.path).is_file();
        (page, worker, shipped, noted)
    };
    let tag =
        r#"<script type="module" src="/.tachyon/tac-locks.js" data-tachyon-runtime></script>"#;
    manifest(r#""locks""#, "https://mail.example.com");
    for native in [false, true] {
        let (page, worker, shipped, noted) = built(native);
        let locks = page.find(tag).expect("the page loads the locks module");
        let runtime = page.find("/.tachyon/tac-client.js").expect("runtime");
        assert!(locks < runtime, "the module loads before the runtime");
        assert!(shipped, "the build writes the locks module");
        assert!(!noted, "an https origin needs no note");
        assert!(
            worker.contains("tac-locks.js"),
            "the module works offline: {worker}"
        );
    }
    manifest(r#""locks""#, "http://mail.example.com");
    assert!(built(false).3, "a plain http origin gets the note");
    manifest(r#""locks""#, "http://localhost:3000");
    assert!(!built(false).3, "localhost is a secure page");
    manifest(r#""share""#, "http://mail.example.com");
    let (page, _, shipped, noted) = built(false);
    assert!(
        !page.contains("tac-locks"),
        "no grant, no locks module: {page}"
    );
    assert!(!shipped && !noted);
}
