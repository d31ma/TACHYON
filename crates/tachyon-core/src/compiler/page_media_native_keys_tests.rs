#![allow(clippy::expect_used)]

use crate::node_driver::NodeDriver;
use std::path::Path;

struct NativeKeysDriver;

impl NativeKeysDriver {
    const FRAMES: &'static str = include_str!("../template/tac-media-frames.js");

    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
import { createCipheriv, createECDH, createHmac } from 'node:crypto'
const posted = []
globalThis.postMessage = (message) => posted.push(message)
;(0, eval)(`${await readFile('./tac-media-frames.js', 'utf8')}\nglobalThis.TacMediaFrames = TacMediaFrames`)
const subtle = crypto.subtle
const hex = (bytes) => Buffer.from(bytes).toString('hex')
const bytes = (text) => Buffer.from(text, 'hex')
const b64 = (value) => Buffer.from(value).toString('base64url')
const unb64 = (text) => Buffer.from(text, 'base64url')
const settle = async () => { for (let round = 0; round < 20; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const kidBytes = (kid) => { const value = Buffer.alloc(8); value.writeBigUInt64BE(BigInt(kid)); return value }
const size = (length) => { const value = Buffer.alloc(2); value.writeUInt16BE(length); return value }
const extract = (salt, input) => createHmac('sha256', salt).update(input).digest()
const expand = (prk, info, length) => createHmac('sha256', prk).update(Buffer.concat([info, Buffer.from([1])])).digest().subarray(0, length)
const labeledExtract = (suite, salt, label, input) => extract(salt, Buffer.concat([Buffer.from('HPKE-v1'), suite, Buffer.from(label), input]))
const labeledExpand = (suite, prk, label, info, length) => expand(prk, Buffer.concat([size(length), Buffer.from('HPKE-v1'), suite, Buffer.from(label), info]), length)
const empty = Buffer.alloc(0)
const seal = (recipient, info, aead, messages, ephemeral = null) => {
  const sender = createECDH('prime256v1')
  if (ephemeral) sender.setPrivateKey(ephemeral)
  else sender.generateKeys()
  const enc = sender.getPublicKey()
  const kem = Buffer.from('KEM\x00\x10', 'latin1')
  const suite = Buffer.from([...Buffer.from('HPKE'), 0, 16, 0, 1, 0, aead])
  const dh = sender.computeSecret(recipient)
  const shared = labeledExpand(kem, labeledExtract(kem, empty, 'eae_prk', dh), 'shared_secret', Buffer.concat([enc, recipient]), 32)
  const schedule = Buffer.concat([Buffer.from([0]), labeledExtract(suite, empty, 'psk_id_hash', empty), labeledExtract(suite, empty, 'info_hash', info)])
  const secret = labeledExtract(suite, shared, 'secret', empty)
  const key = labeledExpand(suite, secret, 'key', schedule, aead === 1 ? 16 : 32)
  const nonce = labeledExpand(suite, secret, 'base_nonce', schedule, 12)
  const sealed = messages.map(([plain, aad], sequence) => {
    const iv = Buffer.from(nonce)
    iv[11] ^= sequence
    const cipher = createCipheriv(aead === 1 ? 'aes-128-gcm' : 'aes-256-gcm', key, iv)
    cipher.setAAD(aad)
    return Buffer.concat([cipher.update(plain), cipher.final(), cipher.getAuthTag()])
  })
  return { enc, sealed }
}
const vector = {
  info: bytes('4f6465206f6e2061204772656369616e2055726e'),
  skEm: bytes('4995788ef4b9d6132b249ce59a77281493eb39af373d236a1fe415cb0c2d7beb'),
  pkRm: bytes('04fe8c19ce0905191ebc298a9245792531f26f0cece2460639e8bc39cb7f706a826a779b4cf969b8a0e539c7f62fb3d30ad6aa8f80e30f1d128aafd68a2ce72ea0'),
  skRm: bytes('f3ce7fdae57e1a310d87f1ebbde6f328be0a99cdbcadf4d6589cf29de4b8ffd2'),
  enc: '04a92719c6195d5085104f469a8b9814d5838ff72b60501e2c4466e5e67b325ac98536d7b61a1af4b78e5b7f951c0900be863c403ce65c9bfcb9382657222d18c4',
  pt: bytes('4265617574792069732074727574682c20747275746820626561757479'),
  ct: ['5ad590bb8baa577f8619db35a36311226a896e7342a6d836d8b7bcd2f20b6c7f9076ac232e3ab2523f39513434', 'fa6f037b47fc21826b610172ca9637e82d6e5801eb31cbd3748271affd4ecb06646e0329cbdf3c3cd655b28e82'],
}
const report = {}
const testSender = seal(vector.pkRm, vector.info, 1, [[vector.pt, Buffer.from('Count-0')], [vector.pt, Buffer.from('Count-1')]], vector.skEm)
report.senderMatchesRfc9180 = hex(testSender.enc) === vector.enc && testSender.sealed.map(hex).join() === vector.ct.join()
const recipientKey = await subtle.importKey('jwk', { kty: 'EC', crv: 'P-256', d: b64(vector.skRm), x: b64(vector.pkRm.subarray(1, 33)), y: b64(vector.pkRm.subarray(33)) }, { name: 'ECDH', namedCurve: 'P-256' }, false, ['deriveBits'])
const context = await TacMediaFrames.hpke(recipientKey, new Uint8Array(vector.pkRm), new Uint8Array(bytes(vector.enc)), new Uint8Array(vector.info), 1)
const opened = []
for (const [sequence, ct] of vector.ct.entries()) opened.push(hex(await subtle.decrypt(TacMediaFrames.opening(context, sequence, new TextEncoder().encode(`Count-${sequence}`)), context.key, bytes(ct))))
report.workerMatchesRfc9180 = opened.every((value) => value === hex(vector.pt))
const suite = Buffer.from([0, 4])
const hkdfBits = async (input, info, bits) => {
  const base = await subtle.importKey('raw', input, 'HKDF', false, ['deriveBits'])
  return Buffer.from(await subtle.deriveBits({ name: 'HKDF', hash: 'SHA-256', salt: new Uint8Array(0), info }, base, bits))
}
const pageBase = (secret) => subtle.importKey('raw', secret, 'HKDF', false, ['deriveKey', 'deriveBits'])
const nativeWrap = async (secret, kid, workerPublic, change = {}) => {
  const frameKey = await hkdfBits(secret, Buffer.concat([Buffer.from('SFrame 1.0 Secret key '), kidBytes(kid), suite]), 128)
  const frameSalt = await hkdfBits(secret, Buffer.concat([Buffer.from('SFrame 1.0 Secret salt '), kidBytes(kid), suite]), 96)
  const info = Buffer.concat([Buffer.from('Tachyon SFrame frame key 1'), kidBytes(change.infoKid ?? kid)])
  const { enc, sealed } = seal(workerPublic, info, 2, [[frameKey, Buffer.concat([Buffer.from('key'), kidBytes(kid)])], [frameSalt, Buffer.concat([Buffer.from('salt'), kidBytes(kid)])]])
  const [key, salt] = sealed
  if (change.key) key[key.length - 1] ^= 1
  if (change.salt) salt[salt.length - 1] ^= 1
  return { enc: b64(enc), key: b64(key), salt: b64(salt) }
}
const otherWorker = () => { const other = createECDH('prime256v1'); other.generateKeys(); return other.getPublicKey() }
const bind = async (keyId) => {
  posted.splice(0)
  await TacMediaFrames.receive({ kind: 'bind', keyId })
  await settle()
  const message = posted.splice(0)[0]
  report.boundMessages.push(message)
  return unb64(message.publicKey)
}
const deliver = async (keyId, wrapped) => {
  posted.splice(0)
  await TacMediaFrames.receive({ kind: 'wrapped', keyId, wrapped })
  await settle()
  return { posted: posted.splice(0), stored: TacMediaFrames.receiving.has(BigInt(keyId)) }
}
report.boundMessages = []
const secret = new Uint8Array(32).fill(7)
report.beforeBind = await deliver(3, await nativeWrap(secret, 3, otherWorker()))
const replaced = await bind(7)
const pendingKey = TacMediaFrames.pending.get(7n).privateKey
report.pendingPrivate = { extractable: pendingKey.extractable, type: pendingKey.type }
const current = await bind(7)
report.freshPerBind = hex(replaced) !== hex(current)
report.publicKey = { length: current.length, first: current[0] }
report.replacedBind = await deliver(7, await nativeWrap(secret, 7, replaced))
const fresh = await bind(7)
const wrapped7 = await nativeWrap(secret, 7, fresh)
report.ready = await deliver(7, wrapped7)
const entry = TacMediaFrames.receiving.get(7n)
report.key = { extractable: entry.key.extractable, usages: [...entry.key.usages].sort(), name: entry.key.algorithm.name, length: entry.key.algorithm.length }
report.pendingAfterUse = TacMediaFrames.pending.size
const page = await TacMediaFrames.derive(await pageBase(secret), 7n)
report.sameSalt = hex(entry.salt) === hex(page.salt)
const metadata = new Uint8Array([1, 2, 3])
const plain = new Uint8Array([9, 8, 7, 6, 5])
const fromNative = await TacMediaFrames.encrypt(entry, metadata, plain)
const fromPage = await TacMediaFrames.encrypt(page, metadata, plain)
report.sameFrame = hex(fromNative) === hex(fromPage)
TacMediaFrames.receiving.set(7n, page)
report.pageOpensNative = hex(await TacMediaFrames.decrypt(metadata, fromNative)) === hex(plain)
TacMediaFrames.receiving.delete(7n)
report.replayed = await deliver(7, wrapped7)
await bind(8)
report.replayedIntoOtherKeyId = await deliver(8, wrapped7)
const wrapCase = async (keyId, change, wrapKid = keyId) => deliver(keyId, await nativeWrap(secret, wrapKid, await bind(keyId), change))
report.refused = {
  changedKey: await wrapCase(11, { key: true }),
  changedSalt: await wrapCase(12, { salt: true }),
  otherInfo: await wrapCase(14, { infoKid: 99 }),
  otherKeyId: await wrapCase(16, {}, 15),
  badEnc: await (async () => { await bind(17); return deliver(17, { ...(await nativeWrap(secret, 17, otherWorker())), enc: 'AAAA' }) })(),
  otherWorker: await (async () => { await bind(13); return deliver(13, await nativeWrap(secret, 13, otherWorker())) })(),
  empty: await (async () => { await bind(18); return deliver(18, {}) })(),
}
report.pendingLeft = TacMediaFrames.pending.size
report.boundShapes = [...new Set(report.boundMessages.map((message) => Object.entries(message).map(([name, value]) => `${name}:${typeof value}`).sort().join(',')))]
delete report.boundMessages
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
fn the_frame_worker_opens_hpke_exactly_as_rfc_9180_says() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = NativeKeysDriver::run();
    assert_eq!(report["senderMatchesRfc9180"], true, "{report}");
    assert_eq!(report["workerMatchesRfc9180"], true, "{report}");
}

#[test]
fn the_frame_worker_unwraps_a_native_frame_key_into_a_key_no_script_can_read() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = NativeKeysDriver::run();
    assert_eq!(
        report["pendingPrivate"],
        serde_json::json!({ "extractable": false, "type": "private" })
    );
    assert_eq!(
        report["freshPerBind"], true,
        "a new key pair for every handover"
    );
    assert_eq!(
        report["publicKey"],
        serde_json::json!({ "length": 65, "first": 4 })
    );
    assert_eq!(
        report["boundShapes"],
        serde_json::json!(["keyId:number,kind:string,publicKey:string"]),
        "the worker posts only its public key"
    );
    assert_eq!(
        report["ready"],
        serde_json::json!({ "posted": [{ "kind": "ready", "keyId": 7, "track": "" }], "stored": true })
    );
    assert_eq!(
        report["key"],
        serde_json::json!({ "extractable": false, "usages": ["decrypt", "encrypt"], "name": "AES-GCM", "length": 128 })
    );
    assert_eq!(report["pendingAfterUse"], 0);
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
    assert_eq!(
        report["beforeBind"],
        NativeKeysDriver::refused(3),
        "{report}"
    );
    assert_eq!(
        report["replacedBind"],
        NativeKeysDriver::refused(7),
        "{report}"
    );
    assert_eq!(report["replayed"], NativeKeysDriver::refused(7), "{report}");
    assert_eq!(
        report["replayedIntoOtherKeyId"],
        NativeKeysDriver::refused(8),
        "{report}"
    );
    let cases = [
        ("changedKey", 11),
        ("changedSalt", 12),
        ("otherWorker", 13),
        ("otherInfo", 14),
        ("otherKeyId", 16),
        ("badEnc", 17),
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
        report["pendingLeft"], 0,
        "every handover key pair is used once"
    );
    assert_eq!(
        report["sendRefused"],
        serde_json::json!([{ "kind": "missing", "keyId": 11, "track": "" }])
    );
    assert_eq!(report["framesWithoutKey"], 0, "no key, no frame");
}
