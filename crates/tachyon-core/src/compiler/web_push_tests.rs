#![allow(clippy::expect_used)]

use super::web_push::WebPush;
use crate::node_driver::NodeDriver;
use std::path::Path;

struct WebPushDriver;

impl WebPushDriver {
    const PAGE_DRIVER: &'static str = r#"import { readFile } from 'node:fs/promises'
const effects = { sent: [], registered: [], unsubscribed: 0 }
const grants = { content: 'push' }
globalThis.document = { visibilityState: 'visible', addEventListener: () => {}, querySelector: (selector) => selector === 'meta[name="tachyon-browser-apis"]' ? grants : null }
globalThis.isSecureContext = true
const activation = { isActive: true }
const manager = { current: null }
class FakeSubscription {
  constructor(serverKey) { this.endpoint = 'https://push.example.com/send/1'; this.options = { applicationServerKey: serverKey.buffer } }
  toJSON() { return { endpoint: this.endpoint, expirationTime: null, keys: { p256dh: 'BPublic', auth: 'secret' } } }
  async unsubscribe() { effects.unsubscribed += 1; manager.current = null; return true }
}
manager.getSubscription = async () => manager.current
manager.subscribe = async ({ userVisibleOnly, applicationServerKey }) => { effects.visible = userVisibleOnly; manager.current = new FakeSubscription(applicationServerKey); return manager.current }
const registration = { active: {}, pushManager: manager }
const serviceWorker = {
  register: async (url, options) => { effects.registered.push([url, options.scope]); return registration },
  getRegistration: async (scope) => effects.registered.length > 0 && scope === '/.tachyon/push/' ? registration : undefined,
}
Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { onLine: true, language: 'en', userActivation: activation, serviceWorker } })
globalThis.PushManager = class {}
globalThis.Notification = class { static permission = 'default'; static answer = 'granted'; static requestPermission = async () => Notification.answer }
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const web = globalThis[Symbol.for('tachyon.tac.web')]
TacWebApis.events = { publish: () => {}, send: (owner, name, value) => effects.sent.push([name, value]) }
const ask = (call, args = []) => JSON.parse(web.host(JSON.stringify({ api: 'push', call, args })))
const settle = () => new Promise((resolve) => setTimeout(resolve, 0))
const key = Buffer.from([4, ...new Array(64).fill(7)]).toString('base64url')
const otherKey = Buffer.from([4, ...new Array(64).fill(9)]).toString('base64url')
const answers = {}
answers.noModule = [ask('supported'), ask('subscribe', [key])]
;(0, eval)(await readFile('./tac-web-push.js', 'utf8'))
answers.supported = ask('supported')
answers.empty = ask('subscription')
await settle()
answers.badKey = ask('subscribe', ['not a key'])
answers.shortKey = ask('subscribe', [Buffer.from([4, 1, 2]).toString('base64url')])
answers.subscribe = ask('subscribe', [key])
await settle()
answers.sameKey = ask('subscribe', [key])
await settle()
answers.otherKey = ask('subscribe', [otherKey])
await settle()
answers.subscription = ask('subscription')
await settle()
answers.unsubscribe = ask('unsubscribe')
await settle()
Notification.answer = 'denied'
answers.denied = ask('subscribe', [key])
await settle()
activation.isActive = false
answers.noGesture = ask('subscribe', [key])
activation.isActive = true
globalThis.isSecureContext = false
answers.insecure = [ask('supported'), ask('subscribe', [key])]
globalThis.isSecureContext = true
globalThis.__tachyonNativeHostCall = () => '{}'
answers.native = [ask('supported'), ask('subscription')]
delete globalThis.__tachyonNativeHostCall
delete globalThis.PushManager
answers.noPushManager = ask('subscribe', [key])
grants.content = 'share'
answers.notGranted = ask('supported')
console.log(JSON.stringify({ answers, effects }))
"#;

    const WORKER_DRIVER: &'static str = r#"import { readFile } from 'node:fs/promises'
const handlers = {}
const effects = { shown: [], opened: [], focused: [], closed: 0 }
const windows = []
globalThis.self = globalThis
globalThis.location = new URL('https://app.example.com/.tachyon/tac-push-sw.js')
self.addEventListener = (name, handler) => { handlers[name] = handler }
self.skipWaiting = async () => true
self.registration = { showNotification: async (title, options) => { effects.shown.push([title, options]) } }
self.clients = { matchAll: async (options) => options.includeUncontrolled ? windows : [], openWindow: async (url) => { effects.opened.push(url) } }
;(0, eval)(await readFile('./tac-push-sw.js', 'utf8'))
const fire = async (name, fields) => { let work = null; handlers[name]({ ...fields, waitUntil: (promise) => { work = promise } }); await work }
const data = (text) => ({ json: () => JSON.parse(text), text: () => text })
const click = (path) => ({ notification: { data: { path }, close: () => { effects.closed += 1 } } })
await fire('push', { data: data('{"title":"New mail","body":"From Ana","path":"/mail/7?from=push#top","tag":"mail-7"}') })
await fire('push', { data: data('Plain words') })
await fire('push', { data: null })
await fire('push', { data: data('{"title":"Away","path":"https://evil.example/x"}') })
await fire('push', { data: data('{"title":"Away","path":"//evil.example/x"}') })
await fire('push', { data: data('{"title":"Away","path":"/\\\\evil.example/x"}') })
windows.push({ url: 'https://app.example.com/mail/7', focus: async () => { effects.focused.push('/mail/7') } })
await fire('notificationclick', click('/mail/7'))
await fire('notificationclick', click('/mail/8'))
await fire('notificationclick', click('https://evil.example/'))
console.log(JSON.stringify({ events: Object.keys(handlers).sort(), effects }))
"#;

    fn run(driver: &str) -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        let [(_, page), (_, worker)] = WebPush::files();
        Self::write(
            root.path(),
            "tac-web.js",
            include_str!("../template/tac-web.js").as_bytes(),
        );
        Self::write(root.path(), "tac-web-push.js", &page);
        Self::write(root.path(), "tac-push-sw.js", &worker);
        Self::write(root.path(), "drive.mjs", driver.as_bytes());
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
}

#[test]
fn a_push_grant_subscribes_through_the_push_worker_and_answers_with_events() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = WebPushDriver::run(WebPushDriver::PAGE_DRIVER);
    let answers = &report["answers"];
    let missing = "Web Push needs a web browser. The native app has no Push API.";
    assert_eq!(
        answers["noModule"],
        serde_json::json!([{"value": false}, {"error": missing}])
    );
    assert_eq!(answers["supported"], serde_json::json!({"value": true}));
    for started in [
        "empty",
        "subscribe",
        "sameKey",
        "otherKey",
        "subscription",
        "unsubscribe",
        "denied",
    ] {
        assert_eq!(
            answers[started],
            serde_json::json!({"value": null}),
            "{started}: {report}"
        );
    }
    let bad_key = "The Web Push server key must be a base64url VAPID public key: 65 bytes that start with 0x04.";
    assert_eq!(answers["badKey"]["error"], bad_key);
    assert_eq!(answers["shortKey"]["error"], bad_key);
    assert_eq!(
        answers["noGesture"]["error"],
        "The push browser API needs a user action. Call it from a click or key handler."
    );
    assert_eq!(answers["insecure"][0], serde_json::json!({"value": false}));
    assert_eq!(
        answers["insecure"][1]["error"],
        "Web Push needs an HTTPS page or localhost."
    );
    assert_eq!(answers["native"][0], serde_json::json!({"value": false}));
    assert_eq!(answers["native"][1]["error"], missing);
    assert!(
        answers["noPushManager"]["error"]
            .as_str()
            .is_some_and(|message| message.contains("added to the Home Screen")),
        "{report}"
    );
    assert_eq!(
        answers["notGranted"]["error"],
        "The push browser API is not granted. Add \"push\" to tac.browserApis in client/manifest.json."
    );
    let subscribed = serde_json::json!(["PushSubscribed", {
        "endpoint": "https://push.example.com/send/1",
        "p256dh": "BPublic",
        "auth": "secret",
        "expirationTime": 0
    }]);
    let effects = &report["effects"];
    assert_eq!(
        effects["sent"],
        serde_json::json!([
            ["PushUnsubscribed", {"endpoint": ""}],
            subscribed,
            subscribed,
            subscribed,
            subscribed,
            ["PushUnsubscribed", {"endpoint": "https://push.example.com/send/1"}],
            ["BrowserCallFailed", {"api": "push", "call": "subscribe", "message": "Notifications are not allowed, so Web Push cannot start."}]
        ]),
        "{report}"
    );
    assert_eq!(
        effects["unsubscribed"], 2,
        "one key change and one unsubscribe: {report}"
    );
    assert_eq!(effects["visible"], true);
    let worker = serde_json::json!(["/.tachyon/tac-push-sw.js", "/.tachyon/push/"]);
    assert_eq!(
        effects["registered"],
        serde_json::json!([worker, worker, worker])
    );
}

#[test]
fn the_push_worker_shows_messages_and_opens_only_same_origin_paths() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = WebPushDriver::run(WebPushDriver::WORKER_DRIVER);
    assert_eq!(
        report["events"],
        serde_json::json!(["install", "notificationclick", "push"])
    );
    let effects = &report["effects"];
    let home =
        |title: &str, body: &str| serde_json::json!([title, {"body": body, "data": {"path": "/"}}]);
    assert_eq!(
        effects["shown"],
        serde_json::json!([
            ["New mail", {"body": "From Ana", "tag": "mail-7", "data": {"path": "/mail/7?from=push#top"}}],
            home("app.example.com", "Plain words"),
            home("app.example.com", ""),
            home("Away", ""),
            home("Away", ""),
            home("Away", "")
        ]),
        "{report}"
    );
    assert_eq!(effects["closed"], 3);
    assert_eq!(effects["focused"], serde_json::json!(["/mail/7"]));
    assert_eq!(
        effects["opened"],
        serde_json::json!(["https://app.example.com/mail/8", "https://app.example.com/"])
    );
}

#[test]
fn only_a_web_build_that_grants_push_ships_the_push_scripts() {
    let root = tempfile::tempdir().expect("project");
    let pages = root.path().join("client/pages");
    std::fs::create_dir_all(&pages).expect("pages");
    std::fs::write(pages.join("tac.html"), "<page-home />").expect("page");
    let component = root.path().join("client/components/page/home");
    std::fs::create_dir_all(&component).expect("component");
    std::fs::write(component.join("tac.html"), "<main>Home</main>").expect("component view");
    let manifest = |grants: &str| {
        std::fs::write(
            root.path().join("client/manifest.json"),
            format!(r#"{{"tac":{{"browserApis":[{grants}]}}}}"#),
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
        let shipped = [".tachyon/tac-web-push.js", ".tachyon/tac-push-sw.js"]
            .map(|path| output.path().join(path).is_file());
        (page, shipped)
    };
    let tag =
        r#"<script type="module" src="/.tachyon/tac-web-push.js" data-tachyon-runtime></script>"#;
    manifest(r#""push""#);
    let (page, shipped) = built(false);
    let push = page.find(tag).expect("the web page loads the push module");
    let runtime = page.find("/.tachyon/tac-client.js").expect("runtime");
    assert!(push < runtime, "{page}");
    assert!(page.contains(r#"content="push""#), "{page}");
    assert_eq!(shipped, [true, true]);
    let (page, shipped) = built(true);
    assert!(
        !page.contains("tac-web-push"),
        "a native page has no Web Push: {page}"
    );
    assert_eq!(shipped, [false, false]);
    manifest(r#""share""#);
    let (page, shipped) = built(false);
    assert!(
        !page.contains("tac-web-push"),
        "no grant, no push module: {page}"
    );
    assert_eq!(shipped, [false, false]);
}
