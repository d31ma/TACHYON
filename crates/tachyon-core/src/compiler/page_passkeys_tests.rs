#![allow(clippy::expect_used)]

use super::page_passkeys::PagePasskeys;
use crate::node_driver::NodeDriver;
use std::path::Path;

struct PasskeysDriver;

impl PasskeysDriver {
    const WEB_RUNTIME: &'static str = include_str!("../template/tac-web.js");

    const DRIVER: &'static str = r#"import { readFile } from 'node:fs/promises'
const grants = { content: 'passkeys' }
const activation = { isActive: true }
globalThis.document = { visibilityState: 'visible', addEventListener: () => {}, querySelector: (selector) => selector === 'meta[name="tachyon-browser-apis"]' ? grants : null }
globalThis.isSecureContext = true
const bytes = (...values) => new Uint8Array(values).buffer
class FakeAttestation {
  clientDataJSON = bytes(1, 2)
  attestationObject = bytes(3, 4, 250)
  getAuthenticatorData() { return bytes(5) }
  getTransports() { return ['internal', 'hybrid'] }
  getPublicKey() { return bytes(6, 7) }
  getPublicKeyAlgorithm() { return -7 }
}
class FakeAssertion {
  clientDataJSON = bytes(1)
  authenticatorData = bytes(8)
  signature = bytes(9, 255)
  userHandle = null
}
class FakeCredential {
  constructor(response) { this.id = 'AQI'; this.rawId = bytes(1, 2); this.type = 'public-key'; this.authenticatorAttachment = 'platform'; this.response = response }
  getClientExtensionResults() { return { credProps: { rk: true }, prf: { results: { first: new Uint8Array([251, 255]) } } } }
}
const asked = []
const browser = { answer: null }
const ceremony = (kind) => (options) => {
  asked.push({ kind, mediation: options.mediation ?? 'modal', challenge: [...options.publicKey.challenge], user: options.publicKey.user ? [...options.publicKey.user.id] : null, listed: (options.publicKey.excludeCredentials ?? options.publicKey.allowCredentials ?? []).map((item) => [...item.id]), rp: options.publicKey.rp?.id ?? options.publicKey.rpId })
  const answer = browser.answer
  if (answer === 'wait') return new Promise((resolve, reject) => options.signal.addEventListener('abort', () => reject(new DOMException('stopped', 'AbortError'))))
  if (typeof answer === 'string') return Promise.reject(new DOMException('rp https://secret.example.com user 9f8e', answer))
  return Promise.resolve(answer)
}
Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { onLine: true, language: 'en', userActivation: activation, credentials: { create: ceremony('create'), get: ceremony('get') } } })
const capabilities = { value: { userVerifyingPlatformAuthenticator: true, conditionalGet: false } }
globalThis.PublicKeyCredential = class {
  static getClientCapabilities = async () => capabilities.value
  static isUserVerifyingPlatformAuthenticatorAvailable = async () => false
  static isConditionalMediationAvailable = async () => true
}
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const web = globalThis[Symbol.for('tachyon.tac.web')]
const sent = []
TacWebApis.events = { publish: () => {}, send: (owner, name, value) => sent.push([owner, name, value]) }
const ask = (owner, call, args = []) => JSON.parse(web.calling(owner, () => web.host(JSON.stringify({ api: 'passkeys', call, args }))))
const settle = async () => { for (let round = 0; round < 10; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const take = () => sent.splice(0)
const b64 = (...values) => Buffer.from(values).toString('base64url')
const creation = JSON.stringify({ rp: { id: 'mail.example.com', name: 'Mail' }, user: { id: b64(7, 7), name: 'ana', displayName: 'Ana' }, challenge: b64(1, 2, 3), pubKeyCredParams: [{ type: 'public-key', alg: -7 }], excludeCredentials: [{ type: 'public-key', id: b64(4) }] })
const request = JSON.stringify({ rpId: 'mail.example.com', challenge: b64(9), allowCredentials: [{ type: 'public-key', id: b64(5, 6) }] })
const report = {}
report.noModule = [ask('early', 'supported'), ask('early', 'create', [creation])]
await settle()
report.noModuleSent = take()
;(0, eval)(await readFile('./tac-passkeys.js', 'utf8'))
report.supported = ask('page', 'supported')
report.check = ask('check-new', 'check')
await settle()
capabilities.value = {}
ask('check-old', 'check')
await settle()
capabilities.value = { userVerifyingPlatformAuthenticator: true }
ask('check-mixed', 'check')
await settle()
browser.answer = new FakeCredential(new FakeAttestation())
report.create = ask('create', 'create', [creation])
await settle()
browser.answer = new FakeCredential(new FakeAssertion())
ask('get', 'get', [request])
await settle()
PublicKeyCredential.parseCreationOptionsFromJSON = (json) => ({ ...json, challenge: new Uint8Array([42]), user: { ...json.user, id: new Uint8Array([43]) } })
PublicKeyCredential.parseRequestOptionsFromJSON = (json) => ({ ...json, challenge: new Uint8Array([44]) })
const assertion = new FakeCredential(new FakeAssertion())
assertion.toJSON = () => ({ id: 'from-browser' })
browser.answer = assertion
ask('parsed-create', 'create', [creation])
await settle()
ask('parsed-get', 'get', [request])
await settle()
delete PublicKeyCredential.parseCreationOptionsFromJSON
delete PublicKeyCredential.parseRequestOptionsFromJSON
for (const name of ['NotAllowedError', 'InvalidStateError', 'SecurityError', 'NotSupportedError', 'AbortError', 'UnknownError', 'Bad name!']) {
  browser.answer = name
  ask(`error ${name}`, 'create', [creation])
  await settle()
}
browser.answer = null
ask('no-credential', 'get', [request])
await settle()
ask('bad-json', 'create', ['{not json'])
ask('array', 'get', ['[]'])
ask('bad-bytes', 'get', [JSON.stringify({ challenge: 'not base64url!' })])
ask('bad-length', 'get', [JSON.stringify({ challenge: 'AAAAA' })])
ask('no-user', 'create', [JSON.stringify({ challenge: b64(1) })])
ask('too-large', 'get', [JSON.stringify({ challenge: b64(1), padding: 'x'.repeat(65536) })])
await settle()
browser.answer = 'wait'
ask('first', 'create', [creation])
ask('second', 'get', [request])
ask('third', 'offer', [request])
await settle()
report.cancel = ask('stopper', 'cancel')
await settle()
ask('offered', 'offer', [request])
await settle()
browser.answer = new FakeCredential(new FakeAssertion())
ask('after-offer', 'get', [request])
await settle()
browser.answer = 'wait'
ask('old-offer', 'offer', [request])
await settle()
browser.answer = new FakeCredential(new FakeAssertion())
ask('new-offer', 'offer', [request])
await settle()
PublicKeyCredential.isConditionalMediationAvailable = async () => false
ask('no-autofill', 'offer', [request])
await settle()
report.sent = take()
report.asked = asked.splice(0)
activation.isActive = false
report.noGesture = [ask('x', 'create', [creation]), ask('x', 'get', [request]), ask('quiet', 'cancel'), ask('quiet', 'offer', [request]), ask('quiet', 'check')]
activation.isActive = true
await settle()
report.quietSent = take()
grants.content = 'share'
report.notGranted = [ask('x', 'supported'), ask('x', 'check'), ask('x', 'create', [creation]), ask('x', 'cancel')]
grants.content = 'passkeys'
globalThis.__tachyonNativeHostCall = async () => JSON.stringify({ value: true })
report.native = ask('native', 'supported')
ask('native', 'check')
ask('native', 'create', [creation])
ask('native', 'offer', [request])
await settle()
report.nativeSent = take()
delete globalThis.__tachyonNativeHostCall
globalThis.isSecureContext = false
report.insecure = ask('insecure', 'supported')
ask('insecure', 'get', [request])
await settle()
report.insecureSent = take()
report.asked.push(...asked.splice(0))
console.log(JSON.stringify(report))
"#;

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        Self::write(root.path(), "tac-web.js", Self::WEB_RUNTIME.as_bytes());
        Self::write(root.path(), "tac-passkeys.js", &PagePasskeys::script());
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

    const ATTESTATION: &'static str = r#"{"id":"AQI","rawId":"AQI","response":{"clientDataJSON":"AQI","authenticatorData":"BQ","transports":["internal","hybrid"],"publicKey":"Bgc","publicKeyAlgorithm":-7,"attestationObject":"AwT6"},"authenticatorAttachment":"platform","clientExtensionResults":{"credProps":{"rk":true},"prf":{"results":{"first":"-_8"}}},"type":"public-key"}"#;
    const ASSERTION: &'static str = r#"{"id":"AQI","rawId":"AQI","response":{"clientDataJSON":"AQ","authenticatorData":"CA","signature":"Cf8"},"authenticatorAttachment":"platform","clientExtensionResults":{"credProps":{"rk":true},"prf":{"results":{"first":"-_8"}}},"type":"public-key"}"#;
    const FROM_BROWSER: &'static str = r#"{"id":"from-browser"}"#;
    const STARTED: &'static str = r#"{"value":null}"#;

    fn started() -> serde_json::Value {
        serde_json::from_str(Self::STARTED).expect("started")
    }

    fn message(reason: &str) -> &'static str {
        match reason {
            "cancelled" => "The person closed the passkey prompt, or chose no passkey.",
            "exists" => "A passkey in excludeCredentials is already on this device.",
            "unsupported" => "This page cannot use passkeys.",
            "refused" => "The relying party does not match this page.",
            "busy" => "Another passkey prompt is already open.",
            "invalid" => "The passkey options are not WebAuthn JSON of at most 65536 bytes.",
            _ => "The passkey call failed.",
        }
    }

    fn failed(owner: &str, call: &str, reason: &str, name: &str) -> serde_json::Value {
        let text = match name {
            "" => String::from(Self::message(reason)),
            _ => format!("{} ({name})", Self::message(reason)),
        };
        serde_json::json!([owner, "PasskeyFailed", {"call": call, "reason": reason, "message": text}])
    }

    fn support(owner: &str, platform: bool, conditional: bool) -> serde_json::Value {
        serde_json::json!([owner, "PasskeySupport", {"platform": platform, "conditional": conditional}])
    }

    fn asserted(owner: &str, credential: &str, conditional: bool) -> serde_json::Value {
        serde_json::json!([owner, "PasskeyAsserted", {"credentialJson": credential, "conditional": conditional}])
    }

    fn created(owner: &str, credential: &str) -> serde_json::Value {
        serde_json::json!([owner, "PasskeyCreated", {"credentialJson": credential}])
    }

    fn asked(
        kind: &str,
        mediation: &str,
        challenge: &[u8],
        user: Option<&[u8]>,
        listed: &[&[u8]],
    ) -> serde_json::Value {
        serde_json::json!({"kind": kind, "mediation": mediation, "challenge": challenge, "user": user, "listed": listed, "rp": "mail.example.com"})
    }
}

#[test]
fn the_page_runtime_answers_passkey_ceremonies_with_passkey_events() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PasskeysDriver::run();
    let failed = PasskeysDriver::failed;
    let assertion = PasskeysDriver::ASSERTION;
    assert_eq!(
        report["sent"],
        serde_json::json!([
            PasskeysDriver::support("check-new", true, false),
            PasskeysDriver::support("check-old", false, true),
            PasskeysDriver::support("check-mixed", true, true),
            PasskeysDriver::created("create", PasskeysDriver::ATTESTATION),
            PasskeysDriver::asserted("get", assertion, false),
            PasskeysDriver::created("parsed-create", PasskeysDriver::FROM_BROWSER),
            PasskeysDriver::asserted("parsed-get", PasskeysDriver::FROM_BROWSER, false),
            failed(
                "error NotAllowedError",
                "create",
                "cancelled",
                "NotAllowedError"
            ),
            failed(
                "error InvalidStateError",
                "create",
                "exists",
                "InvalidStateError"
            ),
            failed("error SecurityError", "create", "refused", "SecurityError"),
            failed(
                "error NotSupportedError",
                "create",
                "unsupported",
                "NotSupportedError"
            ),
            failed("error AbortError", "create", "cancelled", "AbortError"),
            failed("error UnknownError", "create", "failed", "UnknownError"),
            failed("error Bad name!", "create", "failed", ""),
            failed("no-credential", "get", "cancelled", ""),
            failed("bad-json", "create", "invalid", "SyntaxError"),
            failed("array", "get", "invalid", "TypeError"),
            failed("bad-bytes", "get", "invalid", "TypeError"),
            failed("bad-length", "get", "invalid", "TypeError"),
            failed("no-user", "create", "invalid", "TypeError"),
            failed("too-large", "get", "invalid", "TypeError"),
            failed("second", "get", "busy", ""),
            failed("third", "offer", "busy", ""),
            PasskeysDriver::asserted("after-offer", assertion, false),
            PasskeysDriver::asserted("new-offer", assertion, true),
            failed("no-autofill", "offer", "unsupported", "NotSupportedError"),
        ]),
        "a ceremony that Tachyon stopped (cancel, or an offer replaced by a newer call) sends nothing: {report}"
    );
    assert_eq!(report["create"], PasskeysDriver::started());
    assert_eq!(report["cancel"], PasskeysDriver::started());
    let text = report.to_string();
    for secret in ["secret.example.com", "9f8e"] {
        assert!(
            !text.contains(secret),
            "no browser message reaches an event: {report}"
        );
    }
}

#[test]
fn the_page_runtime_asks_the_browser_only_for_valid_calls() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PasskeysDriver::run();
    let create = PasskeysDriver::asked("create", "modal", &[1, 2, 3], Some(&[7, 7]), &[&[4]]);
    let get = PasskeysDriver::asked("get", "modal", &[9], None, &[&[5, 6]]);
    let offer = PasskeysDriver::asked("get", "conditional", &[9], None, &[&[5, 6]]);
    let asked = report["asked"].as_array().expect("asked");
    assert_eq!(
        asked.len(),
        17,
        "busy, invalid and unsupported calls never reach the browser: {report}"
    );
    assert_eq!(asked[0], create, "{report}");
    assert_eq!(asked[1], get, "{report}");
    assert_eq!(asked[2]["challenge"], serde_json::json!([42]), "{report}");
    assert_eq!(asked[2]["user"], serde_json::json!([43]), "{report}");
    assert_eq!(asked[3]["challenge"], serde_json::json!([44]), "{report}");
    assert_eq!(
        asked[12..17],
        [create, offer.clone(), get, offer.clone(), offer],
        "{report}"
    );
}

#[test]
fn the_page_runtime_refuses_passkeys_without_a_grant_a_gesture_or_a_secure_page() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PasskeysDriver::run();
    let started = PasskeysDriver::started();
    let failed = PasskeysDriver::failed;
    assert_eq!(
        report["noModule"],
        serde_json::json!([{"value": true}, started])
    );
    assert_eq!(
        report["noModuleSent"],
        serde_json::json!([["early", "BrowserCallFailed", {"api": "passkeys", "call": "create", "message": "The passkeys browser API did not load on this page."}]])
    );
    assert_eq!(report["supported"], serde_json::json!({"value": true}));
    let gesture =
        "The passkeys browser API needs a user action. Call it from a click or key handler.";
    assert_eq!(
        report["noGesture"],
        serde_json::json!([{"error": gesture}, {"error": gesture}, started, started, started])
    );
    let quiet = report["quietSent"].as_array().expect("quiet");
    assert_eq!(quiet.len(), 2, "{report}");
    assert!(
        quiet.contains(&PasskeysDriver::support("quiet", true, false)),
        "{report}"
    );
    assert!(
        quiet.contains(&failed(
            "quiet",
            "offer",
            "unsupported",
            "NotSupportedError"
        )),
        "{report}"
    );
    let not_granted = "The passkeys browser API is not granted. Add \"passkeys\" to tac.browserApis in client/manifest.json.";
    assert_eq!(
        report["notGranted"],
        serde_json::json!([{"value": true}, {"error": not_granted}, {"error": not_granted}, {"error": not_granted}])
    );
    assert_eq!(report["native"], serde_json::json!({"value": false}));
    assert_eq!(
        report["nativeSent"],
        serde_json::json!([
            failed("native", "create", "unsupported", ""),
            failed("native", "offer", "unsupported", ""),
            PasskeysDriver::support("native", false, false),
        ])
    );
    assert_eq!(report["insecure"], serde_json::json!({"value": false}));
    assert_eq!(
        report["insecureSent"],
        serde_json::json!([failed("insecure", "get", "unsupported", "")])
    );
}

#[test]
fn only_a_build_that_grants_passkeys_ships_the_passkeys_module() {
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
            .any(|note| note.starts_with("Passkeys need a secure page"));
        let page = std::fs::read_to_string(output.path().join("index.html")).expect("page");
        let worker = std::fs::read_to_string(output.path().join("tachyon-sw.js")).expect("sw");
        let shipped = output.path().join(PagePasskeys::PATH).is_file();
        (page, worker, shipped, noted)
    };
    let tag =
        r#"<script type="module" src="/.tachyon/tac-passkeys.js" data-tachyon-runtime></script>"#;
    manifest(r#""passkeys""#, "https://mail.example.com");
    for native in [false, true] {
        let (page, worker, shipped, noted) = built(native);
        assert!(!noted, "an https origin needs no note");
        let passkeys = page.find(tag).expect("the page loads the passkeys module");
        let runtime = page.find("/.tachyon/tac-client.js").expect("runtime");
        assert!(passkeys < runtime, "{page}");
        assert!(page.contains(r#"content="passkeys""#), "{page}");
        assert!(shipped, "native {native}");
        assert!(
            worker.contains("tac-passkeys.js"),
            "the module works offline: {worker}"
        );
        assert!(
            !page.contains("publickey-credentials"),
            "Tachyon writes no Permissions-Policy that blocks WebAuthn: {page}"
        );
    }
    manifest(r#""passkeys""#, "http://mail.example.com");
    assert!(built(false).3, "an http origin gets the secure page note");
    manifest(r#""passkeys""#, "http://localhost:3000");
    assert!(!built(false).3, "localhost is a secure page");
    manifest(r#""share""#, "http://mail.example.com");
    let (page, _, shipped, noted) = built(false);
    assert!(!noted, "no grant, no note");
    assert!(
        !page.contains("tac-passkeys"),
        "no grant, no passkeys module: {page}"
    );
    assert!(!shipped);
}
