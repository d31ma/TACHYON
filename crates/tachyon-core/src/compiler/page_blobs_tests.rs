#![allow(clippy::expect_used)]

use super::page_blobs::PageBlobs;
use crate::native::SiteSettings;
use crate::node_driver::NodeDriver;
use serde_json::json;
use std::path::Path;

struct PageBlobsDriver;

impl PageBlobsDriver {
    const WEB_RUNTIME: &'static str = include_str!("../template/tac-web.js");
    const DEADLINE: std::time::Duration = std::time::Duration::from_mins(2);

    const MEMORY_DATABASE: &'static str = r"const databases = new Map()
const later = (work) => setTimeout(work, 0)
const memory = { quotaFull: false, refuseBlobs: false, opened: 0 }
const database = () => {
  const stores = new Map()
  return {
    createObjectStore: (name) => stores.set(name, new Map()),
    transaction: (names, mode) => {
      const pending = []
      const transaction = { error: null }
      const request = (work) => {
        const asked = {}
        pending.push(() => { asked.result = work(); asked.onsuccess?.() })
        return asked
      }
      const write = (work) => request(() => {
        if (mode !== 'readwrite') throw new DOMException('read only', 'ReadOnlyError')
        if (memory.quotaFull) throw new DOMException('full', 'QuotaExceededError')
        return work()
      })
      transaction.objectStore = (name) => {
        const rows = stores.get(name)
        return {
          get: (key) => request(() => rows.get(key)),
          getAll: () => request(() => [...rows.keys()].sort().map((key) => rows.get(key))),
          put: (value, key) => write(() => {
            if (memory.refuseBlobs && value instanceof Blob) throw new DOMException('Error preparing Blob/File data', 'UnknownError')
            return rows.set(key, value) && key
          }),
          delete: (key) => write(() => rows.delete(key) && undefined),
        }
      }
      later(() => {
        try {
          while (pending.length) pending.shift()()
          transaction.oncomplete?.()
        } catch (error) {
          transaction.error = error
          transaction.onabort?.()
        }
      })
      return transaction
    },
  }
}
globalThis.indexedDB = {
  open: (name) => {
    const asked = {}
    memory.opened += 1
    later(() => {
      const fresh = !databases.has(name)
      if (fresh) databases.set(name, database())
      asked.result = databases.get(name)
      if (fresh) asked.onupgradeneeded?.()
      asked.onsuccess?.()
    })
    return asked
  },
}
";

    const DRIVER: &'static str = r#"import { readFile } from 'node:fs/promises'
import { createHash } from 'node:crypto'
__MEMORY_DATABASE__
const metas = {
  'meta[name="tachyon-browser-apis"]': { content: 'blobs' },
  'meta[name="tachyon-outgoing-hosts"]': { content: 'cdn.example.com' },
  'meta[name="tachyon-blobs"]': { content: '24 16' },
}
Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { userActivation: { isActive: false }, onLine: true } })
globalThis.location = new URL('https://app.example.com/inbox')
globalThis.document = { querySelector: (selector) => metas[selector] ?? null, addEventListener: () => {} }
const fetched = []
globalThis.fetch = async (url, init) => {
  fetched.push({ url, method: init.method, headers: init.headers, credentials: init.credentials, redirect: init.redirect, size: init.body?.size ?? null })
  if (url.includes('/broken')) throw new TypeError('network down')
  if (url.includes('/missing')) return new Response('no', { status: 404 })
  if (url.includes('/large')) return new Response('x'.repeat(20), { headers: { 'content-type': 'text/plain' } })
  if (url.includes('/upload')) return new Response('{"stored":true}', { status: 201 })
  return new Response('world', { headers: { 'content-type': 'image/png' } })
}
const held = new Map([['file-1', new File(['hello'], 'a.txt', { type: 'text/plain' })]])
const filesKey = Symbol.for('tachyon.tac.files')
globalThis[filesKey] = { held: (id) => { if (!held.has(id)) throw new Error(`UPLOAD_FILE: No file has the id '${id}'.`); return held.get(id) } }
;(0, eval)(`${await readFile('./tac-blobs.js', 'utf8')}\nglobalThis.TacBlobs = TacBlobs`)
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const sent = []
TacWebApis.events = { publish: () => {}, send: (owner, name, value) => sent.push([owner?.name ?? 'nobody', name, value]) }
const web = globalThis[Symbol.for('tachyon.tac.web')]
const owner = { name: 'caller' }
const ask = (call, args = []) => JSON.parse(web.calling(owner, () => web.host(JSON.stringify({ api: 'blobs', call, args }))))
const settle = async () => { for (let round = 0; round < 40; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const heard = async () => { await settle(); return sent.splice(0).map(([, name, value]) => [name, value]) }
const sha = (text) => createHash('sha256').update(text).digest('hex')
const report = {}
report.saveHeld = ask('saveHeld', ['file-1', 'notes/a'])
report.saved = await heard()
report.hash = report.saved[0][1].sha256 === sha('hello')
ask('download', ['/files/b', 'notes/b', 'x-amz-meta-name: b'])
report.downloaded = (await heard()).map(([name, value]) => [name, value.key, value.bytes, value.media, value.sha256 === sha('world')])
ask('list', ['notes/'])
report.listed = (await heard()).map(([name, value]) => [name, name === 'BlobFound' ? { ...value, saved: typeof value.saved } : value])
ask('read', ['notes/a', 1, 3])
report.read = await heard()
ask('open', ['notes/a'])
const opened = await heard()
report.opened = [opened[0][0], opened[0][1].url.startsWith('blob:')]
report.url = ask('url', ['notes/a']).value === opened[0][1].url
report.closed = [ask('close', ['notes/a']), ask('url', ['notes/a'])]
const upload = ask('upload', ['notes/a', 'PUT', '/upload/a', 'content-type: text/plain']).value
report.uploaded = (await heard()).map(([name, value]) => [name, value.upload === upload, value.key, value.status, value.body, value.error])
ask('upload', ['notes/none', 'PUT', '/upload/none', ''])
report.uploadMissing = await heard()
ask('download', ['/missing', 'notes/c', ''])
ask('download', ['/broken', 'notes/d', ''])
ask('download', ['/large', 'notes/e', ''])
report.failedDownloads = (await heard()).map(([name, value]) => [name, value.call, value.key, value.reason]).sort((left, right) => left[2].localeCompare(right[2]))
held.set('file-2', new File(['012345678901234'], 'b.bin'))
ask('saveHeld', ['file-2', 'big/c'])
report.quota = await heard()
memory.quotaFull = true
ask('saveHeld', ['file-1', 'notes/f'])
report.browserQuota = (await heard()).map(([name, value]) => [name, value.reason])
memory.quotaFull = false
report.overwrite = ask('saveHeld', ['file-1', 'notes/b'])
report.overwritten = (await heard()).map(([name, value]) => [name, value.bytes])
memory.refuseBlobs = true
ask('saveHeld', ['file-1', 'notes/b'])
report.bufferSaved = (await heard()).map(([name, value]) => [name, value.bytes])
memory.refuseBlobs = false
ask('read', ['notes/b', 0, 5])
report.bufferRead = (await heard()).map(([name, value]) => [name, atob(value.data), value.size])
const counters = globalThis[Symbol.for('tachyon.tac.space')]
report.space = [counters.length, await counters[0]()]
ask('remove', ['notes/a'])
ask('remove', ['notes/a'])
report.removed = await heard()
ask('read', ['notes/a', 0, 1])
report.readRemoved = (await heard()).map(([name, value]) => [name, value.reason, value.message])
const sealKey = Symbol.for('tachyon.tac.blobs.seal')
globalThis[sealKey] = { keyId: 'k1', seal: async (blob) => new Blob(['S', blob], { type: blob.type }), open: async (blob, keyId) => keyId === 'k1' ? blob.slice(1, blob.size, blob.type) : null }
ask('saveHeld', ['file-1', 'sealed/a'])
report.sealedSaved = (await heard()).map(([name, value]) => [name, value.bytes, value.sha256 === sha('hello')])
ask('read', ['sealed/a', 0, 5])
report.sealedRead = (await heard()).map(([name, value]) => [name, atob(value.data), value.size])
delete globalThis[sealKey]
ask('read', ['sealed/a', 0, 5])
report.unsealed = (await heard()).map(([name, value]) => [name, value.reason])
report.refused = {
  empty: ask('saveHeld', ['file-1', '']).error,
  dots: ask('read', ['a/../b', 0, 1]).error,
  control: ask('remove', ['a\nb']).error,
  long: ask('open', ['k'.repeat(513)]).error,
  chunk: ask('read', ['notes/b', 0, 32769]).error,
  file: ask('saveHeld', ['file-9', 'notes/z']).error,
  host: ask('download', ['https://evil.example.org/x', 'notes/z', '']).error,
  header: ask('download', ['/files/x', 'notes/z', 'cookie: a=b']).error,
  method: ask('upload', ['notes/b', 'DELETE', '/upload/x', '']).error,
}
delete globalThis[filesKey]
report.noFiles = ask('saveHeld', ['file-1', 'notes/z']).error
metas['meta[name="tachyon-browser-apis"]'].content = 'files'
report.notGranted = ask('list', ['']).error
metas['meta[name="tachyon-browser-apis"]'].content = 'blobs'
TacBlobs.connection = null
delete globalThis.indexedDB
ask('list', [''])
report.unavailable = (await heard()).map(([name, value]) => [name, value.call, value.reason])
report.fetched = fetched
report.opens = memory.opened
console.log(JSON.stringify(report))
"#;

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("driver folder");
        let driver = Self::DRIVER.replace("__MEMORY_DATABASE__", Self::MEMORY_DATABASE);
        Self::write(root.path(), "drive.mjs", driver.as_bytes());
        Self::write(root.path(), "tac-blobs.js", &PageBlobs::script());
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

    fn assert_kept(report: &serde_json::Value) {
        assert_eq!(report["saveHeld"], json!({ "value": null }));
        assert_eq!(
            report["saved"],
            json!([["BlobSaved", { "key": "notes/a", "bytes": 5, "media": "text/plain", "sha256": report["saved"][0][1]["sha256"] }]])
        );
        assert_eq!(report["hash"], true);
        assert_eq!(
            report["downloaded"],
            json!([["BlobSaved", "notes/b", 5, "image/png", true]])
        );
        assert_eq!(report["listed"][0][0], "BlobFound");
        assert_eq!(report["listed"][0][1]["key"], "notes/a");
        assert_eq!(report["listed"][0][1]["saved"], "number");
        assert_eq!(report["listed"][1][1]["index"], 1);
        assert_eq!(report["listed"][1][1]["count"], 2);
        assert_eq!(
            report["listed"][2],
            json!(["BlobsListed", { "prefix": "notes/", "count": 2, "bytes": 10, "usedBytes": 10, "maxBytes": 24 }])
        );
        assert_eq!(
            report["read"],
            json!([["BlobChunkRead", { "key": "notes/a", "offset": 1, "size": 5, "data": "ZWxs" }]])
        );
        assert_eq!(report["opened"], json!(["BlobOpened", true]));
        assert_eq!(report["url"], true);
        assert_eq!(
            report["closed"],
            json!([{ "value": null }, { "value": "" }])
        );
        assert_eq!(
            report["uploaded"],
            json!([[
                "BlobUploaded",
                true,
                "notes/a",
                201,
                "{\"stored\":true}",
                ""
            ]])
        );
        assert_eq!(
            report["uploadMissing"],
            json!([["BlobFailed", { "call": "upload", "key": "notes/none", "reason": "missing", "message": "No blob has this key." }]])
        );
        assert_eq!(
            report["failedDownloads"],
            json!([
                ["BlobFailed", "download", "notes/c", "status"],
                ["BlobFailed", "download", "notes/d", "network"],
                ["BlobFailed", "download", "notes/e", "size"],
            ])
        );
    }

    fn assert_refused(report: &serde_json::Value) {
        assert_eq!(report["quota"][0][1]["reason"], "quota");
        assert_eq!(report["quota"][0][1]["key"], "big/c");
        assert_eq!(report["browserQuota"], json!([["BlobFailed", "quota"]]));
        assert_eq!(report["overwrite"], json!({ "value": null }));
        assert_eq!(report["overwritten"], json!([["BlobSaved", 5]]));
        assert_eq!(report["bufferSaved"], json!([["BlobSaved", 5]]));
        assert_eq!(report["bufferRead"], json!([["BlobChunkRead", "hello", 5]]));
        assert_eq!(report["space"], json!([1, 10]));
        assert_eq!(
            report["removed"],
            json!([
                ["BlobRemoved", { "key": "notes/a", "bytes": 5 }],
                ["BlobRemoved", { "key": "notes/a", "bytes": 0 }],
            ])
        );
        assert_eq!(
            report["readRemoved"],
            json!([["BlobFailed", "missing", "No blob has this key."]])
        );
        assert_eq!(report["sealedSaved"], json!([["BlobSaved", 5, true]]));
        assert_eq!(report["sealedRead"], json!([["BlobChunkRead", "hello", 5]]));
        assert_eq!(report["unsealed"], json!([["BlobFailed", "unavailable"]]));
        let key_rule = "BLOB_KEY: A blob key is 1 to 512 bytes of text with no control character and no \"..\".";
        for name in ["empty", "dots", "control", "long"] {
            assert_eq!(report["refused"][name], key_rule, "{name}");
        }
        assert_eq!(
            report["refused"]["chunk"],
            "Read 1 to 32768 bytes at an offset of 0 or more."
        );
        assert!(
            report["refused"]["file"]
                .as_str()
                .is_some_and(|error| error.starts_with("UPLOAD_FILE:"))
        );
        assert!(
            report["refused"]["host"]
                .as_str()
                .is_some_and(|error| error.starts_with("DOWNLOAD_ORIGIN: A download comes from"))
        );
        assert!(
            report["refused"]["header"]
                .as_str()
                .is_some_and(|error| error.starts_with("DOWNLOAD_HEADER: A download sends at most"))
        );
        assert_eq!(
            report["refused"]["method"],
            "UPLOAD_REQUEST: An upload uses PUT or POST."
        );
        assert_eq!(
            report["noFiles"],
            "BLOB_FILE: Saving a held file needs \"files\" in tac.browserApis as well."
        );
        assert!(
            report["notGranted"]
                .as_str()
                .is_some_and(|error| error.contains("Add \"blobs\" to tac.browserApis"))
        );
        assert_eq!(
            report["unavailable"],
            json!([["BlobFailed", "list", "unavailable"]])
        );
        assert_eq!(report["opens"], 1);
        let fetched = report["fetched"].as_array().expect("fetched");
        assert_eq!(
            fetched[0],
            json!({ "url": "https://app.example.com/files/b", "method": "GET", "headers": { "x-amz-meta-name": "b" }, "credentials": "omit", "redirect": "error", "size": null })
        );
        assert_eq!(
            fetched[1],
            json!({ "url": "https://app.example.com/upload/a", "method": "PUT", "headers": { "content-type": "text/plain" }, "credentials": "omit", "redirect": "error", "size": 5 })
        );
    }

    fn refusal(manifest: &str) -> String {
        let failure = SiteSettings::from_captured_manifest(Some(manifest.as_bytes()))
            .expect_err("refused manifest");
        format!("{failure:?}")
    }
}

#[test]
fn the_blobs_runtime_keeps_lists_reads_and_uploads_blobs_within_the_quota() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PageBlobsDriver::run();
    PageBlobsDriver::assert_kept(&report);
    PageBlobsDriver::assert_refused(&report);
}

#[test]
fn a_page_loads_the_blobs_runtime_and_its_quota_only_when_the_manifest_grants_blobs() {
    let plain = PageBlobsDriver::site(r#"{"tac":{"browserApis":["clipboard"]}}"#);
    assert_eq!(PageBlobs::tag(&plain), "");
    let granted = PageBlobsDriver::site(
        r#"{"tac":{"browserApis":["blobs"],"outgoing":{"hosts":["cdn.example.com"]}}}"#,
    );
    assert_eq!(granted.outgoing_hosts, ["cdn.example.com"]);
    assert_eq!(
        PageBlobs::tag(&granted),
        "<meta name=\"tachyon-blobs\" content=\"536870912 67108864\"><meta name=\"tachyon-outgoing-hosts\" content=\"cdn.example.com\"><script type=\"module\" src=\"/.tachyon/tac-blobs.js\" data-tachyon-runtime></script>"
    );
    let both = PageBlobsDriver::site(
        r#"{"tac":{"browserApis":["blobs","files"],"storage":{"blobs":{"maxBytes":1000,"maxBlobBytes":10}}}}"#,
    );
    assert_eq!(
        PageBlobs::tag(&both),
        "<meta name=\"tachyon-blobs\" content=\"1000 10\"><script type=\"module\" src=\"/.tachyon/tac-blobs.js\" data-tachyon-runtime></script>"
    );
    let script = String::from_utf8(PageBlobs::script()).expect("text");
    for class in [
        "class TacRandomId",
        "class TacSha256",
        "class TacTransfer",
        "class TacBlobs",
    ] {
        assert!(script.contains(class), "{class}");
    }
    assert!(crate::template::ShippedScript::compacts_safely(&script));
}

#[test]
fn the_manifest_refuses_blob_limits_that_cannot_hold_a_blob() {
    let rule = "tac.storage.blobs needs maxBlobBytes from 1 to maxBytes";
    for manifest in [
        r#"{"tac":{"storage":{"blobs":{"maxBytes":10,"maxBlobBytes":11}}}}"#,
        r#"{"tac":{"storage":{"blobs":{"maxBlobBytes":0}}}}"#,
        r#"{"tac":{"storage":{"blobs":{"maxBytes":1125899906842625}}}}"#,
    ] {
        let refusal = PageBlobsDriver::refusal(manifest);
        assert!(refusal.contains(rule), "{manifest}: {refusal}");
        assert!(
            refusal.contains("Correct client/manifest.json"),
            "{refusal}"
        );
    }
    let unknown = PageBlobsDriver::refusal(r#"{"tac":{"storage":{"blobs":{"maxFiles":1}}}}"#);
    assert!(unknown.contains("maxFiles"), "{unknown}");
    let small = PageBlobsDriver::site(r#"{"tac":{"storage":{"blobs":{"maxBytes":64}}}}"#);
    assert_eq!(small.blob_limits.content(), "64 64");
}
