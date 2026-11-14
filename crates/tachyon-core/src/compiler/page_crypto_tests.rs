#![allow(clippy::expect_used)]

use super::page_crypto_verifier::PageCryptoVerifier;
use super::page_module::PageModule;
use crate::node_driver::NodeDriver;
use serde_json::{Value, json};
use std::path::Path;

struct CryptoDriver;

impl CryptoDriver {
    const WEB_RUNTIME: &'static str = include_str!("../template/tac-web.js");

    const DRIVER: &'static str = r#"import { readFile } from 'node:fs/promises'
import { pbkdf2Sync, hkdfSync } from 'node:crypto'
const grants = { content: 'share' }
globalThis.document = { visibilityState: 'visible', addEventListener: () => {}, querySelector: (selector) => selector === 'meta[name="tachyon-browser-apis"]' ? grants : null }
Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { onLine: true, language: 'en', userActivation: { isActive: false } } })
globalThis.isSecureContext = true
class FakeStore {
  constructor(rows) { this.rows = rows }
  get(name) { return this.done(() => this.rows.get(name)) }
  add(record) { return this.done(() => { if (this.rows.has(record.name)) throw new DOMException('Key already exists in the object store.', 'ConstraintError'); this.rows.set(record.name, record) }) }
  delete(name) { return this.done(() => { this.rows.delete(name) }) }
  clear() { return this.done(() => { this.rows.clear() }) }
  done(work) { const request = {}; this.transaction.work.push(() => { request.result = work() }); return request }
}
class FakeDatabase {
  constructor() { this.rows = new Map(); this.created = [] }
  createObjectStore(name, options) { this.created.push([name, options]) }
  transaction(name, mode) {
    const transaction = { work: [], mode, objectStore: () => { const store = new FakeStore(this.rows); store.transaction = transaction; return store } }
    setTimeout(() => { try { for (const step of transaction.work) step(); transaction.oncomplete() } catch (error) { transaction.error = error; transaction.onerror() } }, 0)
    return transaction
  }
}
const database = new FakeDatabase()
const opened = []
globalThis.indexedDB = { open: (name, version) => { opened.push([name, version]); const request = { result: database }; setTimeout(() => { if (database.created.length === 0) request.onupgradeneeded(); request.onsuccess() }, 0); return request } }
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const web = globalThis[Symbol.for('tachyon.tac.web')]
const sent = []
TacWebApis.events = { publish: () => {}, send: (owner, name, value) => sent.push([owner, name, value]) }
const ask = (owner, call, args = []) => JSON.parse(web.calling(owner, () => web.host(JSON.stringify({ api: 'crypto', call, args }))))
const settle = async () => { for (let round = 0; round < 40; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const take = () => sent.splice(0)
const hex = (text) => Buffer.from(text, 'hex').toString('base64url')
const b64 = (...values) => Buffer.from(values).toString('base64url')
const filled = (size, value) => Buffer.alloc(size, value).toString('base64url')
const answer = async (owner, call, args) => {
  const request = ask(owner, call, args)
  for (let round = 0; round < 100000 && request.value; round += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0))
    const index = sent.findIndex((item) => item[2].request === request.value)
    if (index >= 0) return sent.splice(index, 1)[0]
  }
  return request
}
const fields = async (call, args) => (await answer('app', call, args))[2]
const module = (await readFile('./tac-crypto.js', 'utf8')).replace("'/.tachyon/tac-ed25519.js'", JSON.stringify(new URL('./verifier-counter.mjs', import.meta.url).href))
const report = {}
report.noGrant = ask('app', 'check')
grants.content = 'share crypto'
report.noModule = ask('app', 'check')
;(0, eval)(module)
report.supported = ask('app', 'supported')
report.check = await answer('app', 'check')
report.opened = opened.splice(0)
report.created = database.created
report.pair = await answer('app', 'makeKeyPair', ['device-sign', 'ed25519', true])
const devicePublic = report.pair[2].publicKey
report.publicKey = await fields('publicKey', ['device-sign'])
const signature = (await fields('sign', ['device-sign', b64(1, 2, 3)])).signature
report.signatureBytes = Buffer.from(signature, 'base64url').length
report.verified = (await fields('verify', [devicePublic, b64(1, 2, 3), signature])).valid
report.changed = (await fields('verify', [devicePublic, b64(1, 2, 4), signature])).valid
report.rfc8032 = (await fields('verify', [hex('3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c'), hex('72'), hex('92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00')])).valid
report.shortSignature = (await fields('verify', [devicePublic, b64(1, 2, 3), b64(1)])).valid
const rfc1 = [hex('d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a'), '', hex('e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b')]
report.emptyNative = (await fields('verify', rfc1)).valid
report.loadsAfterNative = globalThis.verifierLoads ?? 0
const realVerify = crypto.subtle.verify
Object.defineProperty(crypto.subtle, 'verify', { configurable: true, value: (algorithm, key, signature, data) => data.byteLength === 0 ? Promise.resolve(false) : realVerify.call(crypto.subtle, algorithm, key, signature, data) })
report.webkitNonEmptyWrong = (await fields('verify', [hex('3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c'), hex('73'), hex('92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00')])).valid
report.loadsAfterNonEmpty = globalThis.verifierLoads ?? 0
report.webkitEmpty = (await fields('verify', rfc1)).valid
const wrongSignature = Buffer.from(rfc1[2], 'base64url'); wrongSignature[40] ^= 1
report.webkitEmptyWrongSignature = (await fields('verify', [rfc1[0], '', wrongSignature.toString('base64url')])).valid
report.webkitEmptyWrongKey = (await fields('verify', [hex('3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c'), '', rfc1[2]])).valid
report.webkitEmptySmallOrderR = (await fields('verify', [rfc1[0], '', hex('01' + '00'.repeat(63))])).valid
report.webkitEmptyShortSignature = (await fields('verify', [rfc1[0], '', b64(1)])).valid
report.loadsAfterEmpty = globalThis.verifierLoads ?? 0
delete crypto.subtle.verify
const emptySignature = (await fields('sign', ['device-sign', ''])).signature
report.emptySignRoundTrip = (await fields('verify', [devicePublic, '', emptySignature])).valid
report.rfc5869 = (await fields('hkdf', [filled(22, 11), hex('000102030405060708090a0b0c'), hex('f0f1f2f3f4f5f6f7f8f9'), 42])).bytes
report.sha256 = (await fields('hash', [Buffer.from('abc').toString('base64url')])).digest
const nistKey = hex('feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308308')
report.nistKey = await fields('importKey', ['nist', nistKey, 'encrypt', false])
const nistSealed = hex('01' + 'cafebabefacedbaddecaf888' + '522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662' + '76fc6ece0f4e1768cddf8853bb2d551b')
const nistAad = hex('feedfacedeadbeeffeedfacedeadbeefabaddad2')
report.nist = Buffer.from((await fields('decrypt', ['nist', nistSealed, nistAad])).data, 'base64url').toString('hex')
report.nistWrongAad = await fields('decrypt', ['nist', nistSealed, b64(1)])
const tampered = Buffer.from(nistSealed, 'base64url'); tampered[20] ^= 1
report.nistTampered = (await fields('decrypt', ['nist', tampered.toString('base64url'), nistAad])).reason
const versioned = Buffer.from(nistSealed, 'base64url'); versioned[0] = 2
report.otherVersion = (await fields('decrypt', ['nist', versioned.toString('base64url'), nistAad])).reason
report.tooShort = (await fields('decrypt', ['nist', filled(28, 1), ''])).reason
const first = (await fields('encrypt', ['nist', b64(7, 7, 7), b64(9)])).sealed
const second = (await fields('encrypt', ['nist', b64(7, 7, 7), b64(9)])).sealed
const firstBytes = Buffer.from(first, 'base64url')
report.sealedShape = [firstBytes.length, firstBytes[0]]
report.noncesDiffer = firstBytes.subarray(1, 13).toString('hex') !== Buffer.from(second, 'base64url').subarray(1, 13).toString('hex')
report.roundTrip = [(await fields('decrypt', ['nist', first, b64(9)])).data, (await fields('decrypt', ['nist', second, b64(9)])).data]
const largest = (await fields('encrypt', ['nist', filled(32768, 5), ''])).sealed
report.largest = Buffer.from((await fields('decrypt', ['nist', largest, ''])).data, 'base64url').length
const alice = (await fields('makeKeyPair', ['alice', 'x25519', false])).publicKey
const bob = (await fields('makeKeyPair', ['bob', 'x25519', false])).publicKey
report.agreeAlice = await fields('agree', ['alice', bob, b64(1), b64(2), 'alice-bob', 'encrypt', false])
report.agreeBob = await fields('agree', ['bob', alice, b64(1), b64(2), 'bob-alice', 'encrypt', false])
const between = (await fields('encrypt', ['alice-bob', b64(4, 2), ''])).sealed
report.between = (await fields('decrypt', ['bob-alice', between, ''])).data
report.otherInfo = await fields('agree', ['bob', alice, b64(1), b64(3), 'bob-other', 'encrypt', false])
report.otherInfoOpens = (await fields('decrypt', ['bob-other', between, ''])).reason
report.smallOrder = (await fields('agree', ['alice', filled(32, 0), '', '', 'zero', 'derive', false])).reason
const ikm = Buffer.alloc(32, 3)
await fields('importKey', ['root', ikm.toString('base64url'), 'derive', false])
report.deriveKey = await fields('deriveKey', ['root', b64(5), b64(6), 'from-root', 'encrypt', false])
const expected = Buffer.from(hkdfSync('sha256', ikm, Buffer.from([5]), Buffer.from([6]), 32)).toString('base64url')
await fields('importKey', ['expected-root', expected, 'encrypt', false])
const fromRoot = (await fields('encrypt', ['from-root', b64(8), ''])).sealed
report.deriveMatches = (await fields('decrypt', ['expected-root', fromRoot, ''])).data
const salt = filled(16, 4)
report.fromCode = await fields('fromCode', ['backup', 'correct horse battery staple', salt, 'encrypt', true])
const pbkdf2 = pbkdf2Sync('correct horse battery staple', Buffer.alloc(16, 4), 600000, 32, 'sha256').toString('base64url')
await fields('importKey', ['expected-backup', pbkdf2, 'encrypt', false])
const fromBackup = (await fields('encrypt', ['backup', b64(6), ''])).sealed
report.fromCodeMatches = (await fields('decrypt', ['expected-backup', fromBackup, ''])).data
report.signWithAgreement = (await fields('sign', ['alice', 'AQ'])).reason
report.agreeWithSigner = (await fields('agree', ['device-sign', bob, '', '', 'nope', 'encrypt', false])).reason
report.encryptWithDerive = (await fields('encrypt', ['root', '', ''])).reason
report.deriveFromEncrypt = (await fields('deriveKey', ['nist', '', '', 'nope', 'encrypt', false])).reason
report.publicOfSecret = (await fields('publicKey', ['nist'])).reason
report.unknown = await answer('app', 'sign', ['missing', 'AQ'])
report.exists = (await fields('makeKeyPair', ['device-sign', 'x25519', false])).reason
report.existsInMemory = (await fields('importKey', ['nist', nistKey, 'derive', true])).reason
report.events = take()
report.hookSecret = String((await globalThis[Symbol.for('tachyon.tac.crypto.hooks')].key('root'))?.usages)
report.hookEncrypt = (await globalThis[Symbol.for('tachyon.tac.crypto.hooks')].key('backup'))?.algorithm?.name ?? null
report.hookPrivate = await globalThis[Symbol.for('tachyon.tac.crypto.hooks')].key('device-sign')
report.hookMissing = await globalThis[Symbol.for('tachyon.tac.crypto.hooks')].key('missing')
report.stored = [...database.rows.values()].map((record) => [record.name, record.algorithm, record.key.extractable, record.key.type])
report.secretsInEvents = JSON.stringify(report).includes(pbkdf2) || JSON.stringify(report).includes(expected)
report.sync = [
  ['makeKeyPair', ['no spaces', 'ed25519', false]],
  ['makeKeyPair', ['x'.repeat(65), 'ed25519', false]],
  ['makeKeyPair', ['ok', 'p256', false]],
  ['makeKeyPair', ['ok', 'ed25519', 'yes']],
  ['sign', ['ok', 'AQ==']],
  ['sign', ['ok', 'a+b/']],
  ['sign', ['ok', 'A']],
  ['sign', ['ok', filled(32769, 1)]],
  ['decrypt', ['ok', filled(32798, 1), '']],
  ['verify', [filled(31, 1), '', '']],
  ['importKey', ['ok', filled(16, 1), 'encrypt', false]],
  ['importKey', ['ok', filled(32, 1), 'sign', false]],
  ['fromCode', ['ok', 'code', filled(15, 1), 'encrypt', false]],
  ['fromCode', ['ok', '', filled(16, 1), 'encrypt', false]],
  ['fromCode', ['ok', 'x'.repeat(1025), filled(16, 1), 'encrypt', false]],
  ['hkdf', ['', '', '', 0]],
  ['hkdf', ['', '', '', 1025]],
  ['hkdf', ['', '', '', 1.5]],
].map(([call, args]) => ask('app', call, args).error)
report.syncEvents = (await settle(), take())
report.largestSealed = ask('app', 'decrypt', ['ok', filled(32797, 1), '']).value
await settle()
take()
;(0, eval)(module)
report.keptAfterReload = (await fields('publicKey', ['device-sign'])).publicKey === devicePublic
report.unkeptAfterReload = (await fields('sign', ['alice', 'AQ'])).reason
report.forget = await fields('forget', ['device-sign'])
report.forgetMissing = await fields('forget', ['device-sign'])
report.forgotten = (await fields('publicKey', ['device-sign'])).reason
await fields('importKey', ['memory', filled(32, 2), 'encrypt', false])
report.forgetAll = await fields('forgetAll', [])
report.afterForgetAll = [(await fields('encrypt', ['backup', '', ''])).reason, (await fields('encrypt', ['memory', '', ''])).reason, database.rows.size]
report.otherOwner = (await answer('other', 'hash', ['']))[0]
globalThis.__tachyonBrowserApiBridge = ['crypto']
const hosted = []
const replies = { sign: { value: { signature: 'AQ', request: 'spoofed' } }, verify: { value: { reason: 'locked' } }, hash: { error: 'The host failed.' }, forget: { value: { reason: 'odd' } } }
globalThis.__tachyonNativeHostCall = async (name, payload) => { hosted.push([name, JSON.parse(payload)]); return JSON.stringify(replies[name.split('.')[2]]) }
report.hostSupported = ask('app', 'supported')
report.hostSign = await fields('sign', ['device-sign', b64(1)])
report.hostVerify = await fields('verify', [filled(32, 1), 'AQ', ''])
report.hostHash = await fields('hash', [''])
report.hostForget = await fields('forget', ['x'])
report.hosted = hosted
delete globalThis.__tachyonNativeHostCall
delete globalThis.__tachyonBrowserApiBridge
globalThis.isSecureContext = false
report.insecureSupported = ask('app', 'supported')
report.insecureCheck = await fields('check', [])
report.insecureMake = (await fields('makeKeyPair', ['a', 'ed25519', false])).reason
report.insecureHash = (await fields('hash', [''])).reason
globalThis.isSecureContext = true
delete globalThis.indexedDB
;(0, eval)(module)
report.noStoreCheck = (await fields('check', [])).keyStore
report.noStoreKeep = (await fields('makeKeyPair', ['kept', 'ed25519', true])).reason
report.noStoreMemory = (await fields('makeKeyPair', ['held', 'ed25519', false])).key
console.log(JSON.stringify(report))
"#;

    const VERIFIER_COUNTER: &'static str = "globalThis.verifierLoads = (globalThis.verifierLoads ?? 0) + 1\nexport * from './tac-ed25519.mjs'\n";

    fn run() -> Value {
        let root = tempfile::tempdir().expect("root");
        Self::write(root.path(), "tac-web.js", Self::WEB_RUNTIME.as_bytes());
        Self::write(root.path(), "tac-crypto.js", &PageModule::CRYPTO.script());
        Self::write(
            root.path(),
            "tac-ed25519.mjs",
            &PageCryptoVerifier::script(),
        );
        Self::write(
            root.path(),
            "verifier-counter.mjs",
            Self::VERIFIER_COUNTER.as_bytes(),
        );
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

    fn message(reason: &str) -> &'static str {
        match reason {
            "unsupported" => {
                "This page cannot run this crypto call: the algorithm or the key store is missing here."
            }
            "invalid" => "The key or the input does not fit this crypto call.",
            "unknown" => "No key with this name is on this device.",
            "exists" => "A key with this name already exists. Forget it first.",
            "rejected" => "The sealed bytes do not open with this key and AAD.",
            "locked" => "The key store is locked.",
            _ => "The crypto call failed.",
        }
    }

    fn failed(request: &str, call: &str, reason: &str) -> Value {
        json!({"request": request, "call": call, "reason": reason, "message": Self::message(reason)})
    }

    fn reason(report: &Value, name: &str) -> String {
        report[name].as_str().unwrap_or_default().to_owned()
    }
}

#[test]
fn the_page_runtime_answers_crypto_calls_with_known_answers() {
    if !NodeDriver::available() {
        return;
    }
    let report = CryptoDriver::run();
    assert_eq!(
        report["noGrant"]["error"],
        json!(
            "The crypto browser API is not granted. Add \"crypto\" to tac.browserApis in client/manifest.json."
        ),
        "{report}"
    );
    assert_eq!(
        report["noModule"]["error"],
        json!("The crypto browser API did not load on this page.")
    );
    assert_eq!(report["supported"], json!({"value": true}));
    assert_eq!(
        report["check"],
        json!(["app", "CryptoSupport", {"request": "crypto-1", "x25519": true, "ed25519": true, "aesGcm": true, "hkdf": true, "pbkdf2": true, "keyStore": "browser"}])
    );
    assert_eq!(report["opened"], json!([["tachyon-keys", 1]]));
    assert_eq!(report["created"], json!([["keys", {"keyPath": "name"}]]));
    assert_eq!(report["pair"][1], json!("CryptoKeyMade"));
    assert_eq!(report["pair"][2]["key"], json!("device-sign"));
    assert_eq!(
        report["publicKey"]["publicKey"],
        report["pair"][2]["publicKey"]
    );
    assert_eq!(report["signatureBytes"], json!(64));
    assert_eq!(report["verified"], json!(true));
    assert_eq!(report["changed"], json!(false));
    assert_eq!(report["rfc8032"], json!(true), "RFC 8032 test 2");
    assert_eq!(report["shortSignature"], json!(false));
    assert_eq!(
        report["rfc5869"],
        json!("PLJfJfqs1XqQQ09k0DYvKi0tCpDPGlpMXbAtVuzExb80AHII1biHGFhl"),
        "RFC 5869 test 1"
    );
    assert_eq!(
        report["sha256"],
        json!("ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0")
    );
    assert_eq!(
        report["nist"],
        json!(
            "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a721c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39"
        ),
        "NIST GCM test 16"
    );
    assert_eq!(
        report["nistWrongAad"],
        CryptoDriver::failed("crypto-22", "decrypt", "rejected")
    );
    for name in ["nistTampered", "otherVersion", "tooShort"] {
        assert_eq!(CryptoDriver::reason(&report, name), "rejected", "{name}");
    }
    assert_eq!(report["sealedShape"], json!([32, 1]));
    assert_eq!(report["noncesDiffer"], json!(true));
    assert_eq!(report["roundTrip"], json!(["BwcH", "BwcH"]));
    assert_eq!(report["largest"], json!(32768));
    assert_eq!(report["agreeAlice"]["publicKey"], json!(""));
    assert_eq!(report["agreeBob"]["key"], json!("bob-alice"));
    assert_eq!(report["between"], json!("BAI"));
    assert_eq!(CryptoDriver::reason(&report, "otherInfoOpens"), "rejected");
    assert_eq!(CryptoDriver::reason(&report, "smallOrder"), "invalid");
    assert_eq!(report["deriveKey"]["key"], json!("from-root"));
    assert_eq!(report["deriveMatches"], json!("CA"));
    assert_eq!(report["fromCode"]["key"], json!("backup"));
    assert_eq!(report["fromCodeMatches"], json!("Bg"));
    for name in [
        "signWithAgreement",
        "agreeWithSigner",
        "encryptWithDerive",
        "deriveFromEncrypt",
        "publicOfSecret",
    ] {
        assert_eq!(CryptoDriver::reason(&report, name), "invalid", "{name}");
    }
    assert_eq!(report["unknown"][1], json!("CryptoFailed"));
    assert_eq!(report["unknown"][2]["reason"], json!("unknown"));
    assert_eq!(
        report["unknown"][2]["message"],
        json!("No key with this name is on this device.")
    );
    assert_eq!(CryptoDriver::reason(&report, "exists"), "exists");
    assert_eq!(CryptoDriver::reason(&report, "existsInMemory"), "exists");
    assert_eq!(report["events"], json!([]), "every answer was taken");
}

#[test]
fn the_page_runtime_keeps_keys_out_of_events_and_on_the_device() {
    if !NodeDriver::available() {
        return;
    }
    let report = CryptoDriver::run();
    assert_eq!(report["secretsInEvents"], json!(false), "{report}");
    assert_eq!(report["hookSecret"], json!("deriveKey,deriveBits"));
    assert_eq!(report["hookEncrypt"], json!("AES-GCM"));
    assert_eq!(report["hookPrivate"], Value::Null, "never a private key");
    assert_eq!(report["hookMissing"], Value::Null);
    assert_eq!(
        report["stored"],
        json!([
            ["device-sign", "ed25519", false, "private"],
            ["backup", "encrypt", false, "secret"]
        ])
    );
    assert_eq!(report["keptAfterReload"], json!(true));
    assert_eq!(
        CryptoDriver::reason(&report, "unkeptAfterReload"),
        "unknown"
    );
    assert_eq!(
        report["forget"].as_object().map(serde_json::Map::len),
        Some(1)
    );
    assert!(report["forgetMissing"]["request"].is_string());
    assert!(report["forgetMissing"].get("reason").is_none());
    assert_eq!(CryptoDriver::reason(&report, "forgotten"), "unknown");
    assert!(report["forgetAll"].get("reason").is_none());
    assert_eq!(report["afterForgetAll"], json!(["unknown", "unknown", 0]));
    assert_eq!(report["otherOwner"], json!("other"));
}

#[test]
fn the_page_runtime_refuses_bad_crypto_input_at_once() {
    if !NodeDriver::available() {
        return;
    }
    let report = CryptoDriver::run();
    let name = "A key name is 1 to 64 letters, digits, dots, dashes or underscores.";
    let padding = "Crypto bytes are base64url text without padding.";
    let code = "The code is 1 to 1024 bytes of text.";
    let length = "The length is a whole number from 1 to 1024.";
    assert_eq!(
        report["sync"],
        json!([
            name,
            name,
            "The algorithm is one of: x25519, ed25519.",
            "keep is true or false.",
            padding,
            padding,
            padding,
            "Crypto bytes are at most 32768 bytes here.",
            "Crypto bytes are at most 32797 bytes here.",
            "A public key is 32 bytes.",
            "A secret is 32 bytes.",
            "The use is one of: encrypt, derive.",
            "The salt is at least 16 bytes.",
            code,
            code,
            length,
            length,
            length
        ]),
        "{report}"
    );
    assert_eq!(
        report["syncEvents"],
        json!([]),
        "a refused call sends nothing"
    );
    assert!(report["largestSealed"].is_string());
}

#[test]
fn the_page_runtime_sends_crypto_calls_to_a_native_host_that_answers_them() {
    if !NodeDriver::available() {
        return;
    }
    let report = CryptoDriver::run();
    assert_eq!(report["hostSupported"], json!({"value": true}));
    assert_eq!(report["hostSign"]["signature"], json!("AQ"));
    assert_ne!(report["hostSign"]["request"], json!("spoofed"));
    assert_eq!(report["hostVerify"]["reason"], json!("locked"));
    assert_eq!(
        report["hostVerify"]["message"],
        json!("The key store is locked.")
    );
    assert_eq!(report["hostHash"]["reason"], json!("failed"));
    assert_eq!(report["hostForget"]["reason"], json!("failed"));
    assert_eq!(
        report["hosted"],
        json!([
            ["browserApi.crypto.sign", ["device-sign", "AQ"]],
            [
                "browserApi.crypto.verify",
                ["AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE", "AQ", ""]
            ],
            ["browserApi.crypto.hash", [""]],
            ["browserApi.crypto.forget", ["x"]]
        ])
    );
}

#[test]
fn the_page_runtime_answers_unsupported_on_an_insecure_page_or_without_a_key_store() {
    if !NodeDriver::available() {
        return;
    }
    let report = CryptoDriver::run();
    assert_eq!(report["insecureSupported"], json!({"value": false}));
    let check = &report["insecureCheck"];
    for flag in ["x25519", "ed25519", "aesGcm", "hkdf", "pbkdf2"] {
        assert_eq!(check[flag], json!(false), "{flag}: {report}");
    }
    assert_eq!(check["keyStore"], json!("none"));
    assert_eq!(CryptoDriver::reason(&report, "insecureMake"), "unsupported");
    assert_eq!(CryptoDriver::reason(&report, "insecureHash"), "unsupported");
    assert_eq!(report["noStoreCheck"], json!("none"));
    assert_eq!(CryptoDriver::reason(&report, "noStoreKeep"), "unsupported");
    assert_eq!(report["noStoreMemory"], json!("held"));
}

#[test]
fn only_a_build_that_grants_crypto_ships_the_crypto_module() {
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
            .any(|note| note.starts_with("Crypto needs a secure page"));
        let page = std::fs::read_to_string(output.path().join("index.html")).expect("page");
        let worker = std::fs::read_to_string(output.path().join("tachyon-sw.js")).expect("sw");
        let shipped = [PageModule::CRYPTO.path, PageCryptoVerifier::PATH]
            .iter()
            .all(|path| output.path().join(path).is_file());
        (page, worker, shipped, noted)
    };
    let tag =
        r#"<script type="module" src="/.tachyon/tac-crypto.js" data-tachyon-runtime></script>"#;
    manifest(r#""crypto""#, "https://mail.example.com");
    for native in [false, true] {
        let (page, worker, shipped, noted) = built(native);
        assert!(!noted, "an https origin needs no note");
        let crypto = page.find(tag).expect("the page loads the crypto module");
        let runtime = page.find("/.tachyon/tac-client.js").expect("runtime");
        assert!(crypto < runtime, "{page}");
        assert!(page.contains(r#"content="crypto""#), "{page}");
        assert!(shipped, "native {native}");
        assert!(
            worker.contains("tac-crypto.js"),
            "the module works offline: {worker}"
        );
    }
    manifest(r#""crypto""#, "http://mail.example.com");
    assert!(built(false).3, "an http origin gets the secure page note");
    manifest(r#""crypto""#, "http://localhost:3000");
    assert!(!built(false).3, "localhost is a secure page");
    manifest(r#""share""#, "http://mail.example.com");
    let (page, _, shipped, noted) = built(false);
    assert!(!noted, "no grant, no note");
    assert!(
        !page.contains("tac-crypto"),
        "no grant, no crypto module: {page}"
    );
    assert!(!shipped);
    assert!(
        !page.contains("tac-ed25519"),
        "the verifier never loads up front: {page}"
    );
}

#[test]
fn the_page_runtime_checks_an_empty_message_with_the_strict_verifier_only_when_webcrypto_says_no() {
    if !NodeDriver::available() {
        return;
    }
    let report = CryptoDriver::run();
    assert_eq!(report["emptyNative"], json!(true), "{report}");
    assert_eq!(report["loadsAfterNative"], json!(0), "WebCrypto said yes");
    assert_eq!(report["webkitNonEmptyWrong"], json!(false));
    assert_eq!(
        report["loadsAfterNonEmpty"],
        json!(0),
        "a message with bytes"
    );
    assert_eq!(report["webkitEmpty"], json!(true), "RFC 8032 test 1");
    for name in [
        "webkitEmptyWrongSignature",
        "webkitEmptyWrongKey",
        "webkitEmptySmallOrderR",
        "webkitEmptyShortSignature",
    ] {
        assert_eq!(report[name], json!(false), "{name}: {report}");
    }
    assert_eq!(report["loadsAfterEmpty"], json!(1), "loaded once");
    assert_eq!(report["emptySignRoundTrip"], json!(true));
}

#[test]
fn the_shipped_verifier_keeps_the_pinned_noble_bytes_and_its_license() {
    use sha2::{Digest, Sha256};
    let pinned = "a4f631d1876b5d9e43a86cd71194ca27b5e1451edf4ba4db9a418f40a475569a";
    let vendored = include_bytes!("../template/vendor/noble-ed25519.js");
    assert_eq!(super::hex_digest(Sha256::digest(vendored)), pinned);
    let shipped = PageCryptoVerifier::script();
    assert!(shipped.ends_with(vendored));
    let notice = String::from_utf8_lossy(&shipped[..shipped.len() - vendored.len()]).into_owned();
    assert!(notice.contains("noble-ed25519 3.2.0"), "{notice}");
    assert!(notice.contains("MIT License"), "{notice}");
    assert!(include_str!("../template/vendor/noble-ed25519.LICENSE").contains("MIT License"));
}
