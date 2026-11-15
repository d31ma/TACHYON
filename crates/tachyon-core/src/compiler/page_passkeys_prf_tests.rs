#![allow(clippy::expect_used)]

use super::page_module::PageModule;
use crate::node_driver::NodeDriver;
use std::path::Path;

struct PasskeysPrfDriver;

impl PasskeysPrfDriver {
    const WEB_RUNTIME: &'static str = include_str!("../template/tac-web.js");

    const DRIVER: &'static str = r#"import { readFile } from 'node:fs/promises'
const grants = { content: 'passkeys crypto' }
globalThis.document = { visibilityState: 'visible', addEventListener: () => {}, querySelector: (selector) => selector === 'meta[name="tachyon-browser-apis"]' ? grants : null }
globalThis.isSecureContext = true
const b64 = (...values) => Buffer.from(values).toString('base64url')
const filled = (value) => new Uint8Array(32).fill(value)
class FakeAssertion {
  clientDataJSON = new Uint8Array([1]).buffer
  authenticatorData = new Uint8Array([8]).buffer
  signature = new Uint8Array([9]).buffer
  userHandle = null
}
class FakeCredential {
  constructor(results, browserJson) {
    this.id = 'AQI'; this.rawId = new Uint8Array([1, 2]).buffer; this.type = 'public-key'; this.response = new FakeAssertion(); this.results = results
    if (browserJson) this.toJSON = () => ({ id: 'from-browser', clientExtensionResults: { prf: { results: { first: Buffer.from(results.first).toString('base64url') } } } })
  }
  getClientExtensionResults() { return this.results ? { prf: { enabled: true, results: this.results } } : {} }
}
const asked = []
const browser = { answer: null }
const ceremony = (kind) => (options) => {
  asked.push({ kind, prf: JSON.parse(JSON.stringify(options.publicKey.extensions?.prf ?? null, (key, value) => value instanceof Uint8Array ? [...value] : value)) })
  return Promise.resolve(browser.answer)
}
Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { onLine: true, language: 'en', userActivation: { isActive: true }, credentials: { create: ceremony('create'), get: ceremony('get') } } })
globalThis.PublicKeyCredential = class {
  static getClientCapabilities = async () => ({})
  static isConditionalMediationAvailable = async () => true
}
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const web = globalThis[Symbol.for('tachyon.tac.web')]
const sent = []
TacWebApis.events = { publish: () => {}, send: (owner, name, value) => sent.push([owner, name, value]) }
const ask = (owner, api, call, args = []) => JSON.parse(web.calling(owner, () => web.host(JSON.stringify({ api, call, args }))))
const heard = async (owner) => {
  for (let round = 0; round < 2000 && !sent.some((item) => item[0] === owner); round += 1) await new Promise((resolve) => setTimeout(resolve, 2))
  const index = sent.findIndex((item) => item[0] === owner)
  return index < 0 ? null : sent.splice(index, 1)[0]
}
const request = JSON.stringify({ rpId: 'mail.example.com', challenge: b64(9) })
const creation = JSON.stringify({ rp: { id: 'mail.example.com', name: 'Mail' }, user: { id: b64(7), name: 'ana', displayName: 'Ana' }, challenge: b64(1), pubKeyCredParams: [{ type: 'public-key', alg: -7 }], extensions: { prf: { eval: { first: b64(4, 4) } } } })
const report = { calls: {}, events: {} }
report.calls.noCryptoModule = ask('early', 'passkeys', 'get', [request, 'backup-prf'])
;(0, eval)(await readFile('./tac-crypto.js', 'utf8'))
;(0, eval)(await readFile('./tac-passkeys.js', 'utf8'))
const hooks = globalThis[Symbol.for('tachyon.tac.crypto.hooks')]
const first = filled(7)
const second = filled(9)
browser.answer = new FakeCredential({ first, second }, true)
report.calls.get = ask('both', 'passkeys', 'get', [request, 'backup-prf'])
report.events.both = await heard('both')
report.zeroed = [first.every((byte) => byte === 0), second.every((byte) => byte === 0)]
report.hookFirst = (await hooks.key('backup-prf'))?.algorithm?.name ?? null
report.hookSecond = (await hooks.key('backup-prf.second'))?.algorithm?.name ?? null
const salt = b64(...new Uint8Array(16).fill(3))
const info = b64(5)
ask('derive', 'crypto', 'deriveKey', ['backup-prf', salt, info, 'backup', 'encrypt', false])
report.events.derive = await heard('derive')
ask('by-hand', 'crypto', 'hkdf', [b64(...filled(7)), salt, info, 32])
const byHand = (await heard('by-hand'))[2].bytes
ask('import', 'crypto', 'importKey', ['by-hand', byHand, 'encrypt', false])
await heard('import')
ask('seal', 'crypto', 'encrypt', ['backup', b64(1, 2, 3), ''])
const sealed = (await heard('seal'))[2].sealed
ask('open', 'crypto', 'decrypt', ['by-hand', sealed, ''])
report.events.open = await heard('open')
browser.answer = new FakeCredential({ first: filled(6) }, false)
ask('first-only', 'passkeys', 'offer', [request, 'first-only'])
report.events.firstOnly = await heard('first-only')
browser.answer = new FakeCredential(null, false)
ask('no-results', 'passkeys', 'get', [request, 'never'])
report.events.noResults = await heard('no-results')
report.hookNever = await hooks.key('never')
browser.answer = new FakeCredential({ first: filled(5) }, false)
ask('no-name', 'passkeys', 'get', [request, ''])
report.events.noName = await heard('no-name')
ask('taken-key', 'crypto', 'importKey', ['taken', b64(...filled(1)), 'derive', false])
await heard('taken-key')
const takenBefore = await hooks.key('taken')
browser.answer = new FakeCredential({ first: filled(2) }, false)
ask('taken', 'passkeys', 'get', [request, 'taken'])
report.events.taken = await heard('taken')
report.takenKept = (await hooks.key('taken')) === takenBefore
ask('second-taken-key', 'crypto', 'importKey', ['pair.second', b64(...filled(1)), 'derive', false])
await heard('second-taken-key')
browser.answer = new FakeCredential({ first: filled(2), second: filled(3) }, false)
ask('second-taken', 'passkeys', 'create', [creation, 'pair'])
report.events.secondTaken = await heard('second-taken')
report.hookPair = await hooks.key('pair')
report.calls.badNames = [ask('x', 'passkeys', 'get', [request, 'bad name!']), ask('x', 'passkeys', 'get', [request, 'a'.repeat(58)]), ask('x', 'passkeys', 'get', [request, null])]
browser.answer = new FakeCredential({ first: filled(4), second: filled(4) }, false)
report.calls.longestName = ask('longest', 'passkeys', 'get', [request, 'a'.repeat(57)])
report.events.longest = await heard('longest')
grants.content = 'passkeys'
report.calls.noCryptoGrant = ask('x', 'passkeys', 'create', [creation, 'backup-prf'])
report.calls.noPrfWithoutCrypto = ask('plain', 'passkeys', 'get', [request, ''])
report.events.plain = await heard('plain')
grants.content = 'passkeys crypto'
asked.splice(0)
const requestWithPrf = JSON.stringify({ rpId: 'mail.example.com', challenge: b64(9), extensions: { prf: { eval: { first: b64(1, 2), second: b64(3) }, evalByCredential: { [b64(5, 6)]: { first: b64(7) } } } } })
browser.answer = null
ask('decode-get', 'passkeys', 'get', [requestWithPrf, ''])
await heard('decode-get')
ask('decode-create', 'passkeys', 'create', [creation, ''])
await heard('decode-create')
ask('bad-input', 'passkeys', 'get', [JSON.stringify({ challenge: b64(9), extensions: { prf: { eval: { first: 'not base64url!' } } } }), ''])
report.events.badInput = await heard('bad-input')
report.asked = asked.splice(0)
report.left = sent.splice(0)
console.log(JSON.stringify(report))
"#;

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        Self::write(root.path(), "tac-web.js", Self::WEB_RUNTIME.as_bytes());
        Self::write(
            root.path(),
            "tac-passkeys.js",
            &PageModule::PASSKEYS.script(),
        );
        Self::write(root.path(), "tac-crypto.js", &PageModule::CRYPTO.script());
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

    const ASSERTION: &'static str = r#"{"id":"AQI","rawId":"AQI","response":{"clientDataJSON":"AQ","authenticatorData":"CA","signature":"CQ"},"clientExtensionResults":{"prf":{"enabled":true}},"type":"public-key"}"#;
    const NO_EXTENSIONS: &'static str = r#"{"id":"AQI","rawId":"AQI","response":{"clientDataJSON":"AQ","authenticatorData":"CA","signature":"CQ"},"clientExtensionResults":{},"type":"public-key"}"#;
    const FROM_BROWSER: &'static str =
        r#"{"id":"from-browser","clientExtensionResults":{"prf":{}}}"#;
    const PRF_NAME_RULE: &'static str = "A PRF key name is 1 to 57 letters, digits, dots, dashes or underscores, so that the name with .second is a key name too.";
    const KEY_EXISTS: &'static str = "A Crypto key with the PRF key name is already on this device. Forget it first. (KeyExistsError)";

    fn asserted(owner: &str, credential: &str, keys: [&str; 2]) -> serde_json::Value {
        serde_json::json!([owner, "PasskeyAsserted", {"credentialJson": credential, "conditional": false, "prfKey": keys[0], "prfSecondKey": keys[1]}])
    }

    fn base64url(bytes: &[u8]) -> String {
        use base64::Engine;
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }
}

#[test]
fn a_passkey_prf_result_becomes_a_crypto_key_that_the_service_never_sees() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PasskeysPrfDriver::run();
    let events = &report["events"];
    assert_eq!(report["calls"]["get"], serde_json::json!({"value": null}));
    assert_eq!(
        events["both"],
        PasskeysPrfDriver::asserted(
            "both",
            PasskeysPrfDriver::FROM_BROWSER,
            ["backup-prf", "backup-prf.second"]
        ),
        "{report}"
    );
    assert_eq!(
        report["zeroed"],
        serde_json::json!([true, true]),
        "the page clears the PRF bytes after the import: {report}"
    );
    assert_eq!(report["hookFirst"], "HKDF", "{report}");
    assert_eq!(report["hookSecond"], "HKDF", "{report}");
    assert_eq!(events["derive"][1], "CryptoKeyMade", "{report}");
    assert_eq!(
        events["open"][2]["data"],
        PasskeysPrfDriver::base64url(&[1, 2, 3]),
        "a key derived from the PRF key equals HKDF over the first PRF result: {report}"
    );
    let text = report["events"].to_string();
    for secret in [[7_u8; 32], [9; 32], [6; 32], [5; 32]] {
        assert!(
            !text.contains(&PasskeysPrfDriver::base64url(&secret)),
            "no PRF result reaches the service: {report}"
        );
    }
}

#[test]
fn a_passkey_without_prf_results_or_without_a_prf_key_name_makes_no_crypto_key() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PasskeysPrfDriver::run();
    let events = &report["events"];
    let mut first_only = PasskeysPrfDriver::asserted(
        "first-only",
        PasskeysPrfDriver::ASSERTION,
        ["first-only", ""],
    );
    first_only[2]["conditional"] = serde_json::json!(true);
    assert_eq!(events["firstOnly"], first_only, "{report}");
    assert_eq!(
        events["noResults"],
        PasskeysPrfDriver::asserted("no-results", PasskeysPrfDriver::NO_EXTENSIONS, ["", ""]),
        "{report}"
    );
    assert_eq!(report["hookNever"], serde_json::Value::Null, "{report}");
    assert_eq!(
        events["noName"],
        PasskeysPrfDriver::asserted("no-name", PasskeysPrfDriver::ASSERTION, ["", ""]),
        "{report}"
    );
    assert_eq!(
        report["calls"]["noPrfWithoutCrypto"],
        serde_json::json!({"value": null})
    );
    assert_eq!(
        events["plain"],
        PasskeysPrfDriver::asserted("plain", PasskeysPrfDriver::ASSERTION, ["", ""]),
        "an empty PRF key name needs no crypto grant: {report}"
    );
    let longest = "a".repeat(57);
    let longest_second = format!("{longest}.second");
    assert_eq!(
        events["longest"],
        PasskeysPrfDriver::asserted(
            "longest",
            PasskeysPrfDriver::ASSERTION,
            [&longest, &longest_second]
        ),
        "a 57-letter name and its 64-letter second name fit: {report}"
    );
    assert_eq!(
        report["calls"]["longestName"],
        serde_json::json!({"value": null})
    );
    assert_eq!(report["left"], serde_json::json!([]), "{report}");
}

#[test]
fn a_taken_or_invalid_prf_key_name_is_refused_and_keeps_the_old_key() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PasskeysPrfDriver::run();
    let events = &report["events"];
    assert_eq!(
        events["taken"],
        serde_json::json!(["taken", "PasskeyFailed", {"call": "get", "reason": "keyExists", "message": PasskeysPrfDriver::KEY_EXISTS}]),
        "{report}"
    );
    assert_eq!(report["takenKept"], true, "{report}");
    assert_eq!(
        events["secondTaken"],
        serde_json::json!(["second-taken", "PasskeyFailed", {"call": "create", "reason": "keyExists", "message": PasskeysPrfDriver::KEY_EXISTS}]),
        "{report}"
    );
    assert_eq!(
        report["hookPair"],
        serde_json::Value::Null,
        "a taken second name makes neither key: {report}"
    );
    let rule = serde_json::json!({"error": PasskeysPrfDriver::PRF_NAME_RULE});
    assert_eq!(
        report["calls"]["badNames"],
        serde_json::json!([rule, rule, rule])
    );
    assert_eq!(
        report["calls"]["noCryptoGrant"],
        serde_json::json!({"error": "The crypto browser API is not granted. Add \"crypto\" to tac.browserApis in client/manifest.json."})
    );
    assert_eq!(
        report["calls"]["noCryptoModule"],
        serde_json::json!({"error": "The crypto browser API did not load on this page."})
    );
}

#[test]
fn the_fallback_converter_decodes_prf_inputs_for_the_browser() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PasskeysPrfDriver::run();
    assert_eq!(PasskeysPrfDriver::base64url(&[5, 6]), "BQY");
    assert_eq!(
        report["asked"],
        serde_json::json!([
            {"kind": "get", "prf": {"eval": {"first": [1, 2], "second": [3]}, "evalByCredential": {"BQY": {"first": [7]}}}},
            {"kind": "create", "prf": {"eval": {"first": [4, 4]}}},
        ]),
        "{report}"
    );
    assert_eq!(
        report["events"]["badInput"][2]["reason"], "invalid",
        "{report}"
    );
}
