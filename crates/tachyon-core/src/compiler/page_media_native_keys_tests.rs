#![allow(clippy::expect_used)]

use crate::node_driver::NodeDriver;
use std::path::Path;

struct NativeKeysDriver;

impl NativeKeysDriver {
    const FRAMES: &'static str = include_str!("../template/tac-media-frames.js");

    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
const posted = []
globalThis.postMessage = (message) => posted.push(message)
;(0, eval)(`${await readFile('./tac-media-frames.js', 'utf8')}\nglobalThis.TacMediaFrames = TacMediaFrames`)
const subtle = crypto.subtle
const hex = (bytes) => Buffer.from(bytes).toString('hex')
const b64 = (bytes) => Buffer.from(bytes).toString('base64url')
const unb64 = (text) => new Uint8Array(Buffer.from(text, 'base64url'))
const settle = async () => { for (let round = 0; round < 20; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const kidBytes = (kid) => { const bytes = Buffer.alloc(8); bytes.writeBigUInt64BE(BigInt(kid)); return bytes }
const suite = Buffer.from([0, 4])
const hkdfBits = async (input, info, bits) => {
  const base = await subtle.importKey('raw', input, 'HKDF', false, ['deriveBits'])
  return new Uint8Array(await subtle.deriveBits({ name: 'HKDF', hash: 'SHA-256', salt: new Uint8Array(0), info }, base, bits))
}
const pageBase = (secret) => subtle.importKey('raw', secret, 'HKDF', false, ['deriveKey', 'deriveBits'])
const nativeWrap = async (secret, kid, workerPublic, change = {}) => {
  const frameKey = await hkdfBits(secret, Buffer.concat([Buffer.from('SFrame 1.0 Secret key '), kidBytes(kid), suite]), 128)
  const frameSalt = await hkdfBits(secret, Buffer.concat([Buffer.from('SFrame 1.0 Secret salt '), kidBytes(kid), suite]), 96)
  const pair = await subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, true, ['deriveBits'])
  const nativePublic = new Uint8Array(await subtle.exportKey('raw', pair.publicKey))
  const worker = await subtle.importKey('raw', workerPublic, { name: 'ECDH', namedCurve: 'P-256' }, false, [])
  const shared = new Uint8Array(await subtle.deriveBits({ name: 'ECDH', public: worker }, pair.privateKey, 256))
  const info = Buffer.concat([Buffer.from('Tachyon SFrame key wrap 1'), kidBytes(change.infoKid ?? kid), nativePublic, workerPublic])
  const wrapKey = await subtle.importKey('raw', await hkdfBits(shared, info, 256), 'AES-GCM', false, ['encrypt'])
  const seal = async (label, plain) => {
    const nonce = crypto.getRandomValues(new Uint8Array(12))
    const additionalData = Buffer.concat([Buffer.from(label), kidBytes(kid)])
    return Buffer.concat([nonce, new Uint8Array(await subtle.encrypt({ name: 'AES-GCM', iv: nonce, additionalData }, wrapKey, plain))])
  }
  const key = await seal('key', frameKey)
  const salt = await seal('salt', frameSalt)
  if (change.key) key[key.length - 1] ^= 1
  if (change.salt) salt[salt.length - 1] ^= 1
  return { publicKey: b64(nativePublic), key: b64(key), salt: b64(salt) }
}
const otherWorker = async () => {
  const pair = await subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, true, ['deriveBits'])
  return new Uint8Array(await subtle.exportKey('raw', pair.publicKey))
}
const deliver = async (keyId, wrapped) => {
  posted.splice(0)
  await TacMediaFrames.receive({ kind: 'wrapped', keyId, wrapped })
  await settle()
  return { posted: posted.splice(0), stored: TacMediaFrames.receiving.has(BigInt(keyId)) }
}
const secret = new Uint8Array(32).fill(7)
const report = {}
report.beforeBind = await deliver(3, await nativeWrap(secret, 3, await otherWorker()))
await TacMediaFrames.receive({ kind: 'bind' })
await settle()
await TacMediaFrames.receive({ kind: 'bind' })
await settle()
report.bound = posted.splice(0)
const workerPublic = unb64(report.bound[0].publicKey)
report.publicKey = { length: workerPublic.length, first: workerPublic[0], same: report.bound[0].publicKey === report.bound[1]?.publicKey }
report.ready = await deliver(7, await nativeWrap(secret, 7, workerPublic))
const entry = TacMediaFrames.receiving.get(7n)
report.key = { extractable: entry.key.extractable, usages: [...entry.key.usages].sort(), name: entry.key.algorithm.name, length: entry.key.algorithm.length }
const page = await TacMediaFrames.derive(await pageBase(secret), 7n)
report.sameSalt = hex(entry.salt) === hex(page.salt)
const metadata = new Uint8Array([1, 2, 3])
const plain = new Uint8Array([9, 8, 7, 6, 5])
const fromNative = await TacMediaFrames.encrypt(entry, metadata, plain)
const fromPage = await TacMediaFrames.encrypt(page, metadata, plain)
report.sameFrame = hex(fromNative) === hex(fromPage)
TacMediaFrames.receiving.set(7n, page)
report.pageOpensNative = hex(await TacMediaFrames.decrypt(metadata, fromNative)) === hex(plain)
report.refused = {
  changedKey: await deliver(11, await nativeWrap(secret, 11, workerPublic, { key: true })),
  changedSalt: await deliver(12, await nativeWrap(secret, 12, workerPublic, { salt: true })),
  otherWorker: await deliver(13, await nativeWrap(secret, 13, await otherWorker())),
  otherInfo: await deliver(14, await nativeWrap(secret, 14, workerPublic, { infoKid: 99 })),
  otherKeyId: await deliver(16, await nativeWrap(secret, 15, workerPublic)),
  badPublicKey: await deliver(17, { ...(await nativeWrap(secret, 17, workerPublic)), publicKey: 'AAAA' }),
  empty: await deliver(18, {}),
}
posted.splice(0)
TacMediaFrames.receive({ kind: 'send', keyId: 11 })
report.sendRefused = posted.splice(0)
const out = []
await TacMediaFrames.frame({ data: new Uint8Array([1, 2, 3]).buffer }, { enqueue: (value) => out.push(value) }, { side: 'send', kind: 'audio' })
report.framesWithoutKey = out.length
console.log(JSON.stringify(report))
";

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        Self::write(root.path(), "tac-media-frames.js", Self::FRAMES);
        Self::write(root.path(), "drive.mjs", Self::DRIVER);
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

    fn write(root: &Path, name: &str, text: &str) {
        std::fs::write(root.join(name), text).expect("write");
    }

    fn refused(key_id: u64) -> serde_json::Value {
        serde_json::json!({ "posted": [{ "kind": "refused", "keyId": key_id, "track": "" }], "stored": false })
    }
}

#[test]
fn the_frame_worker_unwraps_a_native_frame_key_into_a_key_no_script_can_read() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = NativeKeysDriver::run();
    assert_eq!(report["bound"][0]["kind"], "bound", "{report}");
    assert_eq!(
        report["publicKey"],
        serde_json::json!({ "length": 65, "first": 4, "same": true }),
        "one P-256 key pair per worker"
    );
    assert_eq!(
        report["ready"],
        serde_json::json!({ "posted": [{ "kind": "ready", "keyId": 7, "track": "" }], "stored": true })
    );
    assert_eq!(
        report["key"],
        serde_json::json!({ "extractable": false, "usages": ["decrypt", "encrypt"], "name": "AES-GCM", "length": 128 })
    );
}

#[test]
fn a_native_frame_key_makes_the_same_frames_as_the_page_key_path() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = NativeKeysDriver::run();
    assert_eq!(report["sameSalt"], true, "{report}");
    assert_eq!(report["sameFrame"], true, "{report}");
    assert_eq!(report["pageOpensNative"], true, "{report}");
}

#[test]
fn the_frame_worker_refuses_a_frame_key_it_cannot_unwrap_and_sends_nothing() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = NativeKeysDriver::run();
    assert_eq!(report["beforeBind"], NativeKeysDriver::refused(3), "{report}");
    let cases = [
        ("changedKey", 11),
        ("changedSalt", 12),
        ("otherWorker", 13),
        ("otherInfo", 14),
        ("otherKeyId", 16),
        ("badPublicKey", 17),
        ("empty", 18),
    ];
    for (case, key_id) in cases {
        assert_eq!(
            report["refused"][case],
            NativeKeysDriver::refused(key_id),
            "{case}: {report}"
        );
    }
    assert_eq!(
        report["sendRefused"],
        serde_json::json!([{ "kind": "missing", "keyId": 11, "track": "" }])
    );
    assert_eq!(report["framesWithoutKey"], 0, "no key, no frame");
}
