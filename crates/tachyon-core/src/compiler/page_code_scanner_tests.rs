#![allow(clippy::expect_used)]

use super::page_code_scanner::PageCodeScanner;
use crate::node_driver::NodeDriver;
use std::path::Path;

struct CodeScannerDriver;

impl CodeScannerDriver {
    const WEB_RUNTIME: &'static str = include_str!("../template/tac-web.js");
    const QR_CODE: &'static str = include_str!("../../tests/fixtures/code-scanner/qr.txt");
    const QR_TEXT: &'static str = "tachyon-link:7Q2f-9wXk";
    const MODULE_PACKAGE: &'static str = r#"{"type":"module"}"#;

    const DRIVER: &'static str = r#"import { readFile } from 'node:fs/promises'
globalThis.self = globalThis
const grants = { content: 'camera' }
const reason = { meta: { content: 'Scan the code on your other device' } }
const activation = { isActive: true }
const documentListeners = {}
const camera = { answer: 'stream', devices: [{ kind: 'videoinput' }], release: null, stopped: 0, asked: [], change: null }
const detector = { formats: ['qr_code'], codes: [{ rawValue: '' }, { rawValue: 'from detector' }], made: [] }
const rows = (await readFile('./qr.txt', 'utf8')).trim().split('\n')
const picture = (width, height) => {
  const data = new Uint8ClampedArray(width * height * 4).fill(255)
  const scale = Math.floor(Math.min(width, height) / rows.length)
  const left = Math.floor((width - rows.length * scale) / 2)
  const top = Math.floor((height - rows.length * scale) / 2)
  for (let y = 0; y < rows.length * scale; y += 1) for (let x = 0; x < rows.length * scale; x += 1) {
    if (rows[Math.floor(y / scale)][Math.floor(x / scale)] !== '#') continue
    const at = ((top + y) * width + left + x) * 4
    data.fill(0, at, at + 3)
  }
  return { data, width, height }
}
class FakeElement {
  constructor(tag) {
    this.tag = tag
    this.children = []
    this.attributes = {}
    this.listeners = {}
    this.parent = null
    this.open = false
    this.textContent = ''
    if (tag === 'video') Object.assign(this, { readyState: 4, videoWidth: 1280, videoHeight: 720 })
  }
  setAttribute(name, value) { this.attributes[name] = String(value) }
  append(...items) { for (const item of items) { item.parent = this; this.children.push(item) } }
  remove() { if (this.parent) this.parent.children = this.parent.children.filter((child) => child !== this) }
  attachShadow(options) { this.shadowMode = options.mode; this.shadow = new FakeElement('#shadow'); return this.shadow }
  addEventListener(type, listener) { (this.listeners[type] ??= []).push(listener) }
  dispatch(type) { const event = { prevented: false, preventDefault() { event.prevented = true } }; for (const listener of this.listeners[type] ?? []) listener(event); return event.prevented }
  querySelector(tag) { for (const child of this.children) { if (child.tag === tag) return child; const found = child.querySelector(tag); if (found) return found } return null }
  showModal() { this.open = true }
  close() { this.open = false }
  focus() { camera.focused = this.tag }
  play() { return Promise.resolve() }
  getContext() { const canvas = this; return { drawImage: () => {}, getImageData: (x, y, width, height) => picture(width, height), canvas } }
}
const body = new FakeElement('body')
globalThis.document = {
  body,
  visibilityState: 'visible',
  createElement: (tag) => new FakeElement(tag),
  querySelector: (selector) => selector === 'meta[name="tachyon-browser-apis"]' ? grants : selector === 'meta[name="tachyon-camera-reason"]' ? reason.meta : null,
  addEventListener: (type, listener) => (documentListeners[type] ??= new Set()).add(listener),
  removeEventListener: (type, listener) => documentListeners[type]?.delete(listener),
}
globalThis.isSecureContext = true
const stream = () => ({ getTracks: () => [{ stop: () => { camera.stopped += 1 } }] })
const mediaDevices = {
  getUserMedia: (constraints) => {
    camera.asked.push(constraints)
    if (camera.answer === 'stream') return Promise.resolve(stream())
    if (camera.answer === 'wait') return new Promise((resolve) => { camera.release = () => resolve(stream()) })
    return Promise.reject(new DOMException('camera of ana@example.com', camera.answer))
  },
  enumerateDevices: async () => camera.devices,
  addEventListener: (type, listener) => { camera.change = listener },
}
Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { onLine: true, language: 'en', userActivation: activation, mediaDevices } })
globalThis.BarcodeDetector = class {
  static getSupportedFormats = async () => detector.formats
  constructor(options) { detector.made.push(options) }
  detect() { return Promise.resolve(detector.codes) }
}
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const web = globalThis[Symbol.for('tachyon.tac.web')]
const sent = []
TacWebApis.events = { publish: () => {}, send: (owner, name, value) => sent.push([owner, name, value]) }
const ask = (owner, call) => JSON.parse(web.calling(owner, () => web.host(JSON.stringify({ api: 'codeScanner', call, args: [] }))))
const settle = (milliseconds = 20) => new Promise((resolve) => setTimeout(resolve, milliseconds))
const take = () => sent.splice(0)
const view = () => {
  const host = body.children[0]
  if (!host) return null
  const dialog = host.shadow.querySelector('dialog')
  return {
    host: host.attributes['data-tachyon-code-scanner'] === '',
    mode: host.shadowMode,
    open: dialog.open,
    label: dialog.attributes['aria-label'],
    text: dialog.querySelector('p')?.textContent ?? null,
    button: dialog.querySelector('button').textContent,
    videoHidden: dialog.querySelector('video').attributes['aria-hidden'],
    style: host.shadow.querySelector('style').textContent.includes('safe-area-inset-bottom'),
    focused: camera.focused,
  }
}
const dialog = () => body.children[0].shadow.querySelector('dialog')
const report = {}
report.early = ask('early', 'supported')
await import('./tac-code-scanner.js')
await settle()
report.supported = ask('a', 'supported')
report.detected = ask('a', 'scan')
await settle(400)
report.detectedSent = take()
report.detectedAfter = { stopped: camera.stopped, children: body.children.length, constraints: camera.asked.splice(0), made: detector.made.splice(0), listeners: documentListeners.visibilitychange.size }
camera.answer = 'wait'
report.waiting = ask('b', 'scan')
await settle()
report.view = view()
report.busy = ask('b', 'scan')
await settle()
report.busySent = take()
report.cancel = ask('b', 'cancel')
await settle()
report.cancelSent = take()
camera.stopped = 0
camera.release()
await settle()
report.lateStreamStopped = camera.stopped
report.cancelAgain = ask('b', 'cancel')
await settle()
report.cancelAgainSent = take()
ask('c', 'scan')
await settle()
report.escapePrevented = dialog().dispatch('cancel')
await settle()
report.escapeSent = take()
ask('d', 'scan')
await settle()
dialog().querySelector('button').dispatch('click')
await settle()
report.clickSent = take()
ask('e', 'scan')
await settle()
document.visibilityState = 'hidden'
for (const listener of [...documentListeners.visibilitychange]) listener()
document.visibilityState = 'visible'
await settle()
report.hiddenSent = take()
report.hiddenListeners = documentListeners.visibilitychange.size
for (const name of ['NotAllowedError', 'SecurityError', 'NotFoundError', 'OverconstrainedError', 'NotReadableError', 'AbortError']) {
  camera.answer = name
  ask(name, 'scan')
  await settle()
}
report.errorSent = take()
camera.answer = 'stream'
detector.formats = ['ean_13']
report.decoded = ask('f', 'scan')
await settle(600)
report.decodedSent = take()
report.decodedMade = detector.made.length
reason.meta = null
camera.answer = 'wait'
ask('g', 'scan')
await settle()
report.plainView = view()
ask('g', 'cancel')
await settle()
take()
activation.isActive = false
report.noGesture = [ask('h', 'scan'), ask('h', 'cancel')]
await settle()
report.noGestureSent = take()
activation.isActive = true
grants.content = 'clipboard'
report.notGranted = [ask('i', 'supported'), ask('i', 'scan'), ask('i', 'cancel')]
grants.content = 'camera'
camera.devices = [{ kind: 'audioinput' }]
camera.change()
await settle()
report.noCamera = ask('j', 'supported')
ask('j', 'scan')
await settle()
report.noCameraSent = take()
camera.devices = [{ kind: 'videoinput' }]
camera.change()
await settle()
globalThis.isSecureContext = false
report.insecure = ask('k', 'supported')
globalThis.isSecureContext = true
delete globalThis[Symbol.for('tachyon.tac.codeScanner')]
report.noModule = [ask('l', 'supported'), ask('l', 'scan')]
await settle()
report.noModuleSent = take()
const hostCalls = []
globalThis.__tachyonBrowserApiBridge = ['codeScanner']
globalThis.__tachyonNativeHostCall = async (capability, payload) => {
  hostCalls.push([capability, payload])
  return JSON.stringify({ value: capability.endsWith('.scan') ? ['CodeScanned', { text: 'from the host' }] : null })
}
globalThis.__tachyonCameraPresent = true
report.native = ask('m', 'supported')
ask('m', 'scan')
ask('m', 'cancel')
await settle()
report.nativeSent = take()
report.hostCalls = hostCalls
globalThis.__tachyonCameraPresent = false
report.nativeNoCamera = ask('m', 'supported')
console.log(JSON.stringify(report))
"#;

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        Self::write(root.path(), "package.json", Self::MODULE_PACKAGE.as_bytes());
        Self::write(root.path(), "tac-web.js", Self::WEB_RUNTIME.as_bytes());
        Self::write(
            root.path(),
            "tac-code-scanner.js",
            &PageCodeScanner::script(),
        );
        Self::write(
            root.path(),
            "tac-qr-decoder.js",
            &PageCodeScanner::decoder(),
        );
        Self::write(root.path(), "qr.txt", Self::QR_CODE.as_bytes());
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

    fn started() -> serde_json::Value {
        serde_json::json!({"value": null})
    }

    fn failed(owner: &str, reason: &str) -> serde_json::Value {
        serde_json::json!([owner, "ScanFailed", {"reason": reason}])
    }

    fn scanned(owner: &str, text: &str) -> serde_json::Value {
        serde_json::json!([owner, "CodeScanned", {"text": text}])
    }
}

#[test]
fn the_page_scanner_answers_one_code_and_closes_the_camera() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = CodeScannerDriver::run();
    let scanned = CodeScannerDriver::scanned;
    assert_eq!(report["early"], serde_json::json!({"value": false}));
    assert_eq!(report["supported"], serde_json::json!({"value": true}));
    assert_eq!(report["detected"], CodeScannerDriver::started());
    assert_eq!(
        report["detectedSent"],
        serde_json::json!([scanned("a", "from detector")])
    );
    assert_eq!(
        report["detectedAfter"],
        serde_json::json!({
            "stopped": 1,
            "children": 0,
            "constraints": [{"audio": false, "video": {"facingMode": {"ideal": "environment"}, "width": {"ideal": 1280}}}],
            "made": [{"formats": ["qr_code"]}],
            "listeners": 0
        })
    );
    assert_eq!(report["decoded"], CodeScannerDriver::started());
    assert_eq!(
        report["decodedSent"],
        serde_json::json!([scanned("f", CodeScannerDriver::QR_TEXT)])
    );
    assert_eq!(report["decodedMade"], serde_json::json!(0));
}

#[test]
fn the_page_scanner_draws_its_own_view_and_ends_on_every_cancel() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = CodeScannerDriver::run();
    let failed = CodeScannerDriver::failed;
    assert_eq!(report["waiting"], CodeScannerDriver::started());
    assert_eq!(
        report["view"],
        serde_json::json!({
            "host": true,
            "mode": "closed",
            "open": true,
            "label": "Scan the code on your other device",
            "text": "Scan the code on your other device",
            "button": "Cancel",
            "videoHidden": "true",
            "style": true,
            "focused": "button"
        })
    );
    assert_eq!(report["plainView"]["label"], "Scan a QR code");
    assert_eq!(report["plainView"]["text"], serde_json::Value::Null);
    assert_eq!(report["busy"], CodeScannerDriver::started());
    assert_eq!(
        report["busySent"],
        serde_json::json!([["b", "BrowserCallFailed", {"api": "codeScanner", "call": "scan", "message": "A code scan is already open."}]])
    );
    assert_eq!(report["cancel"], CodeScannerDriver::started());
    assert_eq!(
        report["cancelSent"],
        serde_json::json!([failed("b", "cancelled")])
    );
    assert_eq!(report["lateStreamStopped"], serde_json::json!(1));
    assert_eq!(report["cancelAgain"], CodeScannerDriver::started());
    assert_eq!(report["cancelAgainSent"], serde_json::json!([]));
    assert_eq!(report["escapePrevented"], serde_json::json!(true));
    assert_eq!(
        report["escapeSent"],
        serde_json::json!([failed("c", "cancelled")])
    );
    assert_eq!(
        report["clickSent"],
        serde_json::json!([failed("d", "cancelled")])
    );
    assert_eq!(
        report["hiddenSent"],
        serde_json::json!([failed("e", "cancelled")])
    );
    assert_eq!(report["hiddenListeners"], serde_json::json!(0));
}

#[test]
fn the_page_scanner_names_each_camera_failure_with_one_reason_word() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = CodeScannerDriver::run();
    let failed = CodeScannerDriver::failed;
    assert_eq!(
        report["errorSent"],
        serde_json::json!([
            failed("NotAllowedError", "denied"),
            failed("SecurityError", "denied"),
            failed("NotFoundError", "unsupported"),
            failed("OverconstrainedError", "unsupported"),
            failed("NotReadableError", "failed"),
            failed("AbortError", "failed"),
        ])
    );
    assert!(
        !report.to_string().contains("ana@example.com"),
        "the browser's own error text never reaches the app: {report}"
    );
}

#[test]
fn the_page_scanner_refuses_without_a_grant_a_gesture_a_camera_or_its_module() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = CodeScannerDriver::run();
    let gesture =
        "The camera browser API needs a user action. Call it from a click or key handler.";
    assert_eq!(
        report["noGesture"],
        serde_json::json!([{"error": gesture}, {"value": null}])
    );
    assert_eq!(report["noGestureSent"], serde_json::json!([]));
    let not_granted = "The camera browser API is not granted. Add \"camera\" to tac.browserApis in client/manifest.json.";
    assert_eq!(
        report["notGranted"],
        serde_json::json!([{"value": true}, {"error": not_granted}, {"error": not_granted}])
    );
    assert_eq!(report["noCamera"], serde_json::json!({"value": false}));
    assert_eq!(
        report["noCameraSent"],
        serde_json::json!([CodeScannerDriver::failed("j", "unsupported")])
    );
    assert_eq!(report["insecure"], serde_json::json!({"value": false}));
    assert_eq!(
        report["noModule"],
        serde_json::json!([{"value": false}, CodeScannerDriver::started()])
    );
    assert_eq!(
        report["noModuleSent"],
        serde_json::json!([["l", "BrowserCallFailed", {"api": "codeScanner", "call": "scan", "message": "The code scanner did not load on this page."}]])
    );
}

#[test]
fn an_apple_app_answers_the_scanner_from_its_host() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = CodeScannerDriver::run();
    assert_eq!(report["native"], serde_json::json!({"value": true}));
    assert_eq!(
        report["nativeSent"],
        serde_json::json!([CodeScannerDriver::scanned("m", "from the host")])
    );
    assert_eq!(
        report["hostCalls"],
        serde_json::json!([
            ["browserApi.codeScanner.scan", "{}"],
            ["browserApi.codeScanner.cancel", "{}"]
        ])
    );
    assert_eq!(
        report["nativeNoCamera"],
        serde_json::json!({"value": false})
    );
}

#[test]
fn the_shipped_decoder_keeps_the_pinned_jsqr_bytes_and_its_license() {
    use sha2::{Digest, Sha256};
    let pinned = "bc40c8a15196236b2314db0856f72ca0b49980cd5413b8c852a7349f5fee0859";
    let vendored = include_bytes!("../template/vendor/jsQR.js");
    assert_eq!(super::hex_digest(Sha256::digest(vendored)), pinned);
    let shipped = PageCodeScanner::decoder();
    assert!(shipped.ends_with(vendored));
    let notice = String::from_utf8_lossy(&shipped[..shipped.len() - vendored.len()]).into_owned();
    assert!(notice.contains("jsQR 1.4.0"), "{notice}");
    assert!(notice.contains("Apache License, Version 2.0"), "{notice}");
    assert!(include_str!("../template/vendor/jsQR.LICENSE").contains("Apache License"));
}

#[test]
fn only_a_build_that_grants_the_camera_ships_the_code_scanner() {
    let root = tempfile::tempdir().expect("project");
    let pages = root.path().join("client/pages");
    std::fs::create_dir_all(&pages).expect("pages");
    std::fs::write(pages.join("tac.html"), "<page-home />").expect("page");
    let component = root.path().join("client/components/page/home");
    std::fs::create_dir_all(&component).expect("component");
    std::fs::write(component.join("tac.html"), "<main>Home</main>").expect("component view");
    let manifest = |tac: &str| {
        std::fs::write(
            root.path().join("client/manifest.json"),
            format!(r#"{{"tac":{tac}}}"#),
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
        let worker = std::fs::read_to_string(output.path().join("tachyon-sw.js")).expect("sw");
        let shipped = [PageCodeScanner::PATH, PageCodeScanner::DECODER_PATH]
            .map(|path| output.path().join(path).is_file());
        (page, worker, shipped)
    };
    let tag = r#"<script type="module" src="/.tachyon/tac-code-scanner.js" data-tachyon-runtime></script>"#;
    manifest(
        r#"{"browserApis":["camera"],"application":{"permissions":{"camera":"Scan <codes> & \"link\""}}}"#,
    );
    for native in [false, true] {
        let (page, worker, shipped) = built(native);
        let scanner = page.find(tag).expect("the page loads the scanner");
        let runtime = page.find("/.tachyon/tac-client.js").expect("runtime");
        assert!(scanner < runtime, "{page}");
        assert!(
            page.contains(r#"<meta name="tachyon-camera-reason" content="Scan &lt;codes&gt; &amp; &quot;link&quot;">"#),
            "{page}"
        );
        assert_eq!(shipped, [true, true], "native {native}");
        assert!(worker.contains("tac-code-scanner.js"), "{worker}");
        assert!(worker.contains("tac-qr-decoder.js"), "{worker}");
    }
    manifest(r#"{"browserApis":["camera"]}"#);
    let (page, _, shipped) = built(false);
    assert!(page.contains(tag), "{page}");
    assert!(!page.contains("tachyon-camera-reason"), "{page}");
    assert_eq!(shipped, [true, true]);
    manifest(r#"{"browserApis":["share"]}"#);
    let (page, worker, shipped) = built(false);
    assert!(!page.contains("tac-code-scanner"), "{page}");
    assert!(!worker.contains("tac-qr-decoder"), "{worker}");
    assert_eq!(shipped, [false, false]);
}
