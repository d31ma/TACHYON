#![allow(clippy::expect_used)]

use super::page_files::PageFiles;
use crate::native::SiteSettings;
use crate::node_driver::NodeDriver;
use std::path::Path;

struct PageFilesDriver;

impl PageFilesDriver {
    const WEB_RUNTIME: &'static str = include_str!("../template/tac-web.js");
    const DEADLINE: std::time::Duration = std::time::Duration::from_mins(2);

    const DRIVER: &'static str = r#"import { readFile } from 'node:fs/promises'
import { createHash } from 'node:crypto'
const metas = {
  'meta[name="tachyon-browser-apis"]': { content: 'clipboard files' },
  'meta[name="tachyon-outgoing-hosts"]': { content: 'uploads.example.com *.storage.example.net localhost' },
}
const listeners = {}
const inputs = []
const activation = { isActive: true }
Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { userActivation: activation, onLine: true } })
globalThis.location = new URL('https://app.example.com/notes')
globalThis.document = {
  querySelector: (selector) => metas[selector] ?? null,
  addEventListener: (name, listener) => { listeners[name] = listener },
  createElement: () => ({ handlers: {}, files: [], addEventListener(name, handler) { this.handlers[name] = handler }, click() { inputs.push(this) } }),
}
const fetched = []
globalThis.fetch = async (url, init) => {
  fetched.push({ url, method: init.method, headers: init.headers, credentials: init.credentials, redirect: init.redirect, cache: init.cache, referrerPolicy: init.referrerPolicy, size: init.body.size })
  if (url.includes('/broken')) throw new TypeError('network down')
  if (url.includes('/slow')) return new Promise((resolve, reject) => init.signal.addEventListener('abort', () => reject(init.signal.reason)))
  return new Response(url.includes('/long') ? 'x'.repeat(9000) : '{"stored":true}', { status: 201 })
}
;(0, eval)(await readFile('./tac-files.js', 'utf8'))
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const sent = []
TacWebApis.events = { publish: (name, value) => sent.push(['everyone', name, value]), send: (owner, name, value) => sent.push([owner?.name ?? 'nobody', name, value]) }
const web = globalThis[Symbol.for('tachyon.tac.web')]
const owner = { name: 'caller' }
const ask = (call, args = []) => JSON.parse(web.calling(owner, () => web.host(JSON.stringify({ api: 'files', call, args }))))
const tick = () => new Promise((resolve) => setTimeout(resolve, 0))
const settle = async () => { for (let round = 0; round < 20; round += 1) await tick() }
const sha = (bytes) => createHash('sha256').update(bytes).digest('hex')
const large = new Uint8Array(2621441).map((_, index) => (index * 31) % 251)
const report = {}
report.picked = ask('pick', ['.txt, image/*', true])
const input = inputs[0]
report.input = { accept: input.accept, multiple: input.multiple, type: input.type }
input.files = [new File(['abc'], 'a.txt', { type: 'text/plain' }), new File([''], 'empty.txt'), new File([large], 'large.bin')]
input.handlers.change()
await settle()
const picked = sent.splice(0, 3)
report.pickedEvent = picked.map(([who, name, value]) => [who, name, value.index, value.count])
const [abc, empty, big] = picked.map((event) => event[2])
report.hashes = [abc.sha256 === sha('abc'), empty.sha256 === sha(''), big.sha256 === sha(large)]
report.described = { name: abc.name, size: abc.size, media: abc.media, idLength: abc.id.length }
report.read = ask('read', [abc.id, 1, 2])
ask('read', [big.id, 1048570, 32768])
await settle()
const chunks = sent.splice(0, 2).map((event) => event[2])
const chunk = chunks.find((read) => read.id === abc.id)
const bigChunk = chunks.find((read) => read.id === big.id)
report.chunk = { id: chunk.id === abc.id, offset: chunk.offset, size: chunk.size, text: atob(chunk.data) }
report.bigChunk = Buffer.from(bigChunk.data, 'base64').equals(Buffer.from(large.subarray(1048570, 1081338)))
report.tooMuch = ask('read', [abc.id, 0, 32769]).error
report.negative = ask('read', [abc.id, -1, 1]).error
const send = (url, headers = '', method = '') => ask('upload', [abc.id, method, url, headers])
report.upload = send('/uploads/a', 'Content-Type: text/plain\nx-amz-meta-name: a').value
report.declared = send('https://box.storage.example.net/put', '', 'post').value.length
report.local = send('http://localhost:8080/up').value.length
report.long = send('https://uploads.example.com/long').value.length
report.broken = send('/broken').value.length
const slow = send('/slow').value
report.cancel = ask('cancel', [slow])
await settle()
report.fetched = fetched
report.finished = sent.splice(0).map(([who, name, value]) => [who, name, value.status, value.error, value.truncated, value.body.length])
report.refused = {
  undeclared: send('https://evil.example.org/x').error,
  backslash: send('/\\evil.example.org/x').error,
  slashes: send('//evil.example.org/x').error,
  insecure: send('http://uploads.example.com/x').error,
  bare: send('https://storage.example.net/x').error,
  credentials: send('https://user:pass@uploads.example.com/x').error,
  header: send('/up', 'cookie: a=b').error,
  newline: send('/up', 'authorization: a\r\nb').error,
  method: send('/up', '', 'DELETE').error,
  accept: ask('pick', ['text/plain,evil', false]).error,
}
report.release = ask('release', [abc.id])
report.released = ask('read', [abc.id, 0, 1]).error
const zone = { name: 'avatar', attributes: {}, getAttribute(name) { return name === 'data-tac-drop-zone' ? this.name : null }, setAttribute(name, value) { this.attributes[name] = value }, removeAttribute(name) { delete this.attributes[name] } }
const drag = (files, types = ['Files']) => ({ prevented: false, target: { closest: () => zone }, dataTransfer: { types, files }, preventDefault() { this.prevented = true } })
const over = drag([])
listeners.dragover(over)
report.over = [over.prevented, 'data-tac-drag-over' in zone.attributes]
const text = drag([], ['text/plain'])
listeners.dragover(text)
report.textOver = text.prevented
const dropped = drag([new File(['dropped'], 'd.txt')])
listeners.drop(dropped)
await settle()
const heard = sent.shift()
report.drop = [dropped.prevented, 'data-tac-drag-over' in zone.attributes, heard[0], heard[1], heard[2].zone, heard[2].sha256 === sha('dropped'), heard[2].index, heard[2].count]
const many = drag(Array.from({ length: 33 }, (_, index) => new File([String(index)], `${index}.txt`)))
listeners.drop(many)
await settle()
report.tooMany = sent.shift().slice(1).map((value) => value.message ?? value)
report.cancel = ask('pick', ['', false])
inputs.at(-1).handlers.cancel()
report.cancelled = sent.shift()
activation.isActive = false
report.noGesture = ask('pick', ['', false]).error
report.readWithoutGesture = ask('read', [big.id, 0, 1]).error ?? null
metas['meta[name="tachyon-browser-apis"]'].content = 'clipboard'
report.notGranted = ask('read', [big.id, 0, 1]).error
await settle()
console.log(JSON.stringify(report))
"#;

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("driver folder");
        Self::write(root.path(), "drive.mjs", Self::DRIVER.as_bytes());
        Self::write(root.path(), "tac-files.js", &PageFiles::script());
        Self::write(root.path(), "tac-web.js", Self::WEB_RUNTIME.as_bytes());
        let mut command = NodeDriver::command();
        command.arg("drive.mjs").current_dir(root.path());
        let output = NodeDriver::output_within(&mut command, Self::DEADLINE)
            .expect("node drive.mjs within its deadline");
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

    fn site(manifest: &str) -> SiteSettings {
        SiteSettings::from_captured_manifest(Some(manifest.as_bytes())).expect("manifest")
    }
}

#[test]
fn the_files_runtime_picks_hashes_reads_and_uploads_only_to_declared_hosts() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PageFilesDriver::run();
    assert_eq!(report["picked"], serde_json::json!({ "value": null }));
    assert_eq!(
        report["input"],
        serde_json::json!({ "accept": ".txt,image/*", "multiple": true, "type": "file" })
    );
    assert_eq!(
        report["pickedEvent"],
        serde_json::json!([
            ["caller", "FilePicked", 0, 3],
            ["caller", "FilePicked", 1, 3],
            ["caller", "FilePicked", 2, 3]
        ])
    );
    assert_eq!(report["hashes"], serde_json::json!([true, true, true]));
    assert_eq!(
        report["described"],
        serde_json::json!({ "name": "a.txt", "size": 3, "media": "text/plain", "idLength": 36 })
    );
    assert_eq!(
        report["chunk"],
        serde_json::json!({ "id": true, "offset": 1, "size": 3, "text": "bc" })
    );
    assert_eq!(report["bigChunk"], true);
    assert!(
        report["tooMuch"]
            .as_str()
            .is_some_and(|error| error.contains("1 to 32768 bytes"))
    );
    assert!(report["negative"].is_string());
    assert_eq!(report["upload"].as_str().map(str::len), Some(36));
    let fetched = report["fetched"].as_array().expect("fetches");
    assert_eq!(fetched.len(), 6, "{fetched:?}");
    assert_eq!(
        fetched[0],
        serde_json::json!({
            "url": "https://app.example.com/uploads/a",
            "method": "PUT",
            "headers": { "content-type": "text/plain", "x-amz-meta-name": "a" },
            "credentials": "omit",
            "redirect": "error",
            "cache": "no-store",
            "referrerPolicy": "no-referrer",
            "size": 3
        })
    );
    assert_eq!(fetched[1]["method"], "POST");
    assert_eq!(fetched[1]["url"], "https://box.storage.example.net/put");
    assert_eq!(fetched[2]["url"], "http://localhost:8080/up");
    let finished = report["finished"].as_array().expect("finished uploads");
    let summary: Vec<String> = finished.iter().map(ToString::to_string).collect();
    for expected in [
        r#"["caller","UploadFinished",201,"",false,15]"#,
        r#"["caller","UploadFinished",201,"",true,8192]"#,
        r#"["caller","UploadFinished",0,"UPLOAD_NETWORK",false,0]"#,
        r#"["caller","UploadFinished",0,"CANCELLED",false,0]"#,
    ] {
        assert!(summary.contains(&String::from(expected)), "{summary:?}");
    }
    assert_eq!(finished.len(), 6, "{summary:?}");
    let refused = report["refused"].as_object().expect("refusals");
    for (case, code) in [
        ("undeclared", "UPLOAD_ORIGIN"),
        ("backslash", "UPLOAD_ORIGIN"),
        ("slashes", "UPLOAD_ORIGIN"),
        ("insecure", "UPLOAD_ORIGIN"),
        ("bare", "UPLOAD_ORIGIN"),
        ("credentials", "UPLOAD_ORIGIN"),
        ("header", "UPLOAD_HEADER"),
        ("newline", "UPLOAD_HEADER"),
        ("method", "UPLOAD_REQUEST"),
        ("accept", "comma-separated list"),
    ] {
        assert!(
            refused[case]
                .as_str()
                .is_some_and(|error| error.contains(code)),
            "{case}: {}",
            refused[case]
        );
    }
    assert_eq!(report["release"], serde_json::json!({ "value": null }));
    assert!(
        report["released"]
            .as_str()
            .is_some_and(|error| error.contains("No file has the id"))
    );
    assert_eq!(report["over"], serde_json::json!([true, true]));
    assert_eq!(report["textOver"], false);
    assert_eq!(
        report["drop"],
        serde_json::json!([true, false, "everyone", "FileDropped", "avatar", true, 0, 1])
    );
    assert_eq!(report["tooMany"][0], "BrowserCallFailed");
    assert!(
        report["tooMany"][1]
            .as_str()
            .is_some_and(|error| error.contains("at most 32 at once"))
    );
    assert_eq!(
        report["cancelled"],
        serde_json::json!(["caller", "FilePickCancelled", {}])
    );
    assert!(
        report["noGesture"]
            .as_str()
            .is_some_and(|error| error.contains("needs a user action"))
    );
    assert!(report["readWithoutGesture"].is_null());
    assert!(
        report["notGranted"]
            .as_str()
            .is_some_and(|error| error.contains("Add \"files\" to tac.browserApis"))
    );
}

#[test]
fn a_page_loads_the_files_runtime_only_when_the_manifest_grants_files() {
    let plain = PageFilesDriver::site(
        r#"{"tac":{"browserApis":["clipboard"],"outgoing":{"hosts":["api.example.com"]}}}"#,
    );
    assert!(plain.outgoing_hosts.is_empty());
    assert_eq!(PageFiles::tag(&plain), "");
    let granted = PageFilesDriver::site(
        r#"{"tac":{"browserApis":["files"],"outgoing":{"hosts":["uploads.example.com","*.storage.example.net"]}}}"#,
    );
    assert_eq!(
        granted.outgoing_hosts,
        ["uploads.example.com", "*.storage.example.net"]
    );
    assert_eq!(
        PageFiles::tag(&granted),
        "<meta name=\"tachyon-outgoing-hosts\" content=\"uploads.example.com *.storage.example.net\"><script type=\"module\" src=\"/.tachyon/tac-files.js\" data-tachyon-runtime></script>"
    );
    let script = String::from_utf8(PageFiles::script()).expect("text");
    assert!(script.contains("class TacSha256"));
    assert!(script.contains("class TacFiles"));
    assert!(crate::template::ShippedScript::compacts_safely(
        &String::from_utf8_lossy(&PageFiles::script())
    ));
    let refused = SiteSettings::from_captured_manifest(Some(
        br#"{"tac":{"browserApis":["files"],"outgoing":{"hosts":["https://uploads.example.com"]}}}"#,
    ));
    assert!(refused.is_err());
}
