#![allow(clippy::expect_used)]

use crate::node_driver::NodeDriver;
use std::path::Path;

struct FramesDriver;

impl FramesDriver {
    const FRAMES: &'static str = include_str!("../template/tac-media-frames.js");
    const VECTORS: &'static str = include_str!("page_media_frames_vectors.json");

    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
const posted = []
globalThis.postMessage = (message) => posted.push(message)
;(0, eval)(`${await readFile('./tac-media-frames.js', 'utf8')}\nglobalThis.TacMediaFrames = TacMediaFrames`)
const vectors = JSON.parse(await readFile('./vectors.json', 'utf8'))
const hex = (bytes) => Buffer.from(bytes).toString('hex')
const bytes = (text) => new Uint8Array(Buffer.from(text, 'hex'))
const report = { installed: TacMediaFrames.installed }
report.headerFailures = vectors.headers.filter((vector) => {
  const kid = BigInt('0x' + vector.kid)
  const ctr = BigInt('0x' + vector.ctr)
  const parsed = TacMediaFrames.parse(bytes(vector.header))
  return hex(TacMediaFrames.header(kid, ctr)) !== vector.header || parsed.kid !== kid || parsed.ctr !== ctr || parsed.length * 2 !== vector.header.length
}).length
report.headers = vectors.headers.length
const gcm = vectors.aesGcm
const base = (hexKey) => crypto.subtle.importKey('raw', bytes(hexKey), 'HKDF', false, ['deriveKey', 'deriveBits'])
const entry = await TacMediaFrames.derive(await base(gcm.base_key), BigInt('0x' + gcm.kid))
entry.counter = BigInt('0x' + gcm.ctr)
report.salt = hex(entry.salt) === gcm.sframe_salt
report.nonce = hex(TacMediaFrames.nonce(entry.salt, BigInt('0x' + gcm.ctr))) === gcm.nonce
report.sealed = hex(await TacMediaFrames.encrypt(entry, bytes(gcm.metadata), bytes(gcm.pt))) === gcm.ct
TacMediaFrames.receiving.set(entry.kid, entry)
report.opened = hex(await TacMediaFrames.decrypt(bytes(gcm.metadata), bytes(gcm.ct))) === gcm.pt
TacMediaFrames.receiving.clear()
const frame = (type, data) => ({ type, data: new Uint8Array(data).buffer })
const run = async (side, kind, given) => {
  const out = []
  await TacMediaFrames.frame(given, { enqueue: (value) => out.push(new Uint8Array(value.data)) }, { side, kind, track: 'peer-1.0' })
  return out
}
const keyFrame = [...Array(10).keys()].map((index) => 200 + index).concat([1, 2, 3, 4, 5])
report.noSendKey = (await run('send', 'video', frame('key', keyFrame))).length
await TacMediaFrames.receive({ kind: 'use', keyId: 9, key: await base('000102030405060708090a0b0c0d0e0f') })
await TacMediaFrames.receive({ kind: 'use', keyId: 300, key: await base('0f0e0d0c0b0a09080706050403020100') })
TacMediaFrames.receive({ kind: 'send', keyId: 300 })
report.sendKeyLeftReceiving = TacMediaFrames.receiving.has(300n)
const [sealedKey] = await run('send', 'video', frame('key', keyFrame))
const [sealedDelta] = await run('send', 'video', frame('delta', keyFrame))
const [sealedAudio] = await run('send', 'audio', frame(undefined, [9, 8, 7]))
report.keyFrameClear = hex(sealedKey.subarray(0, 10)) === hex(keyFrame.slice(0, 10))
report.deltaClear = hex(sealedDelta.subarray(0, 3)) === hex(keyFrame.slice(0, 3)) && sealedDelta[3] !== keyFrame[3]
report.audioHeader = hex(sealedAudio.subarray(0, 3))
report.counters = [hex(sealedKey.subarray(10, 13)), hex(sealedDelta.subarray(3, 6))]
TacMediaFrames.receiving.set(300n, await TacMediaFrames.derive(await base('0f0e0d0c0b0a09080706050403020100'), 300n))
const [openedKey] = await run('receive', 'video', frame('key', sealedKey))
const [openedAudio] = await run('receive', 'audio', frame(undefined, sealedAudio))
report.roundTrip = hex(openedKey) === hex(keyFrame) && hex(openedAudio) === '090807'
const changed = sealedKey.slice()
changed[0] ^= 1
report.changedClear = (await run('receive', 'video', frame('key', changed))).length
const cut = sealedAudio.slice(0, 2)
report.cutShort = (await run('receive', 'audio', frame(undefined, cut))).length
TacMediaFrames.sending.counter = TacMediaFrames.largestCounter + 1n
report.exhausted = (await run('send', 'audio', frame(undefined, [1]))).length
TacMediaFrames.forget(300n)
report.forgotten = (await run('receive', 'video', frame('key', sealedKey))).length
report.sendingAfterForget = TacMediaFrames.sending
report.posted = posted
console.log(JSON.stringify(report))
";

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        Self::write(root.path(), "tac-media-frames.js", Self::FRAMES);
        Self::write(root.path(), "vectors.json", Self::VECTORS);
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
}

#[test]
fn the_frame_worker_matches_every_rfc_9605_vector_for_aes_128_gcm() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = FramesDriver::run();
    assert_eq!(report["installed"], false, "node is not a worker");
    assert_eq!(report["headers"], 289);
    assert_eq!(report["headerFailures"], 0);
    for check in ["salt", "nonce", "sealed", "opened"] {
        assert_eq!(report[check], true, "{check}: {report}");
    }
}

#[test]
fn the_frame_worker_keeps_the_codec_header_clear_and_drops_bad_frames() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = FramesDriver::run();
    assert_eq!(report["noSendKey"], 0, "no send key, no plain frame");
    assert_eq!(report["sendKeyLeftReceiving"], false);
    assert_eq!(report["keyFrameClear"], true);
    assert_eq!(report["deltaClear"], true);
    assert_eq!(report["audioHeader"], "92012c", "KID 300, CTR 2");
    assert_eq!(report["counters"], serde_json::json!(["90012c", "91012c"]));
    assert_eq!(report["roundTrip"], true);
    assert_eq!(report["changedClear"], 0, "a changed clear byte fails");
    assert_eq!(report["cutShort"], 0);
    assert_eq!(report["forgotten"], 0);
    assert_eq!(report["sendingAfterForget"], serde_json::Value::Null);
    assert_eq!(report["exhausted"], 0);
    assert_eq!(
        report["posted"],
        serde_json::json!([
            {"kind": "ready", "keyId": 9, "track": ""},
            {"kind": "ready", "keyId": 300, "track": ""},
            {"kind": "rejected", "keyId": 0, "track": "peer-1.0"},
            {"kind": "rejected", "keyId": 0, "track": "peer-1.0"},
            {"kind": "exhausted", "keyId": 300, "track": ""},
            {"kind": "missing", "keyId": 300, "track": "peer-1.0"},
        ])
    );
}
