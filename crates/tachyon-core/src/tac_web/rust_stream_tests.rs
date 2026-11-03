#![allow(clippy::expect_used)]

use super::rust_marker_proof_tests::RustPage;

struct RustStreams;

impl RustStreams {
    const FILES: [(&'static str, &'static str); 5] = [
        (
            "client/models/Tick.rs",
            "#[derive(Clone, Debug, Default)]\npub struct Tick {\n    pub sku: String,\n    pub level: i64,\n}\n",
        ),
        (
            "client/models/ChatMessage.rs",
            "#[derive(Clone, Debug, Default)]\npub struct ChatMessage {\n    pub text: String,\n}\n",
        ),
        (
            "client/remotes/TicksClient.rs",
            "use crate::models::Tick;\n\n#[HTTPClient(\"/ticks\")]\ntrait TicksClient {\n    #[StreamRequest]\n    fn receive(&self, tick: Tick);\n}\n",
        ),
        (
            "client/remotes/ChatClient.rs",
            "use crate::models::ChatMessage;\n\n#[HTTPClient(\"/chat\")]\ntrait ChatClient {\n    #[SocketRequest]\n    fn receive(&self, message: ChatMessage);\n}\n",
        ),
        (
            "client/services/TicksService.rs",
            "#[Service]\npub struct TicksService {\n    #[Needs]\n    ticks: std::sync::Arc<TicksClient>,\n    #[Needs]\n    chat: std::sync::Arc<ChatClient>,\n    pub level: i64,\n    pub said: String,\n}\n\n\
         impl TicksService {\n    #[OnEvent]\n    fn heard(&mut self, tick: Tick) {\n        self.level = tick.level;\n    }\n\n    \
         #[OnEvent]\n    fn answered(&mut self, message: ChatMessage) {\n        self.said = message.text;\n    }\n\n    \
         pub async fn say(&mut self, text: String) -> BehaviorResult<()> {\n        self.chat.send(ChatMessage { text }).await\n    }\n}\n",
        ),
    ];

    const DRIVER: &'static str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         globalThis.location = { href: 'http://127.0.0.1:4000/page' }\n\
         const opened = []\n\
         globalThis.EventSource = class { constructor(url) { this.url = String(url); this.closed = false; opened.push(this) } addEventListener() {} close() { this.closed = true } }\n\
         globalThis.WebSocket = class { constructor(url) { this.url = String(url); this.readyState = 0; this.sent = []; this.closed = false; opened.push(this) } send(text) { this.sent.push(text) } close() { this.closed = true } }\n\
         const { default: TicksService, __tachyon_event_plan__: ticksPlan } = await import('./TicksService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         let failures = 0\n\
         const published = []\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { failures += 1 } })\n\
         bus.attach({ receive: (value) => published.push(value) }, { listeners: [{ method: 'receive', event: 'Tick' }] }, {})\n\
         const ticks = new TicksService({})\n\
         bus.attach(ticks, ticksPlan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(ticks)\n\
         const stream = opened.find((source) => !('sent' in source)), socket = opened.find((source) => 'sent' in source)\n\
         const queued = [ticks.say('early')]\n\
         for (let index = 0; index < 15; index += 1) queued.push(ticks.say(`queued ${index}`))\n\
         let refused = ''\n\
         try { await ticks.say('overflow') } catch (error) { refused = `${error.code}: ${error.message}` }\n\
         await Promise.all(queued)\n\
         socket.readyState = 1\n\
         socket.onopen()\n\
         stream.onmessage({ data: JSON.stringify({ sku: 'anvil', level: 3 }), lastEventId: '' })\n\
         stream.onmessage({ data: JSON.stringify({ sku: 'anvil', level: 'high' }), lastEventId: '' })\n\
         socket.onmessage({ data: JSON.stringify({ text: 'hello' }) })\n\
         await ticks.say('late')\n\
         const report = { urls: opened.map((source) => source.url).sort(), level: ticks.level, said: ticks.said, published, failures, refused, sent: socket.sent }\n\
         bus.dispose(ticks)\n\
         await new Promise((resolve) => setTimeout(resolve, 50))\n\
         report.closed = opened.map((source) => source.closed)\n\
         report.afterDispose = opened.length\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";
}

#[tokio::test]
async fn a_rust_service_receives_its_client_streams_as_events_and_sends_on_its_socket() {
    if !RustPage::ready() {
        eprintln!("skipping: the wasm32 Rust target or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let service = ("ticks", "TicksService");
    RustPage::publish(root.path(), &RustStreams::FILES, service).await;
    let report = RustPage::drive(root.path(), RustStreams::DRIVER);
    assert_eq!(
        report["urls"],
        serde_json::json!(["/ticks", "ws://127.0.0.1:4000/chat"]),
        "{report}"
    );
    assert_eq!(report["level"], 3, "{report}");
    assert_eq!(report["said"], "hello", "{report}");
    assert_eq!(
        report["published"],
        serde_json::json!([{"sku": "anvil", "level": 3}]),
        "{report}"
    );
    assert_eq!(report["failures"], 1, "{report}");
    assert!(
        report["refused"].as_str().is_some_and(|message| message
            == "HTTP_NETWORK: Web service ticks: The socket message was not sent."),
        "{report}"
    );
    let sent = report["sent"].as_array().expect("sent");
    assert_eq!(sent.len(), 17, "{report}");
    assert_eq!(sent[0], "{\"text\":\"early\"}", "{report}");
    assert_eq!(sent[15], "{\"text\":\"queued 14\"}", "{report}");
    assert_eq!(sent[16], "{\"text\":\"late\"}", "{report}");
    assert_eq!(
        report["closed"],
        serde_json::json!([true, true]),
        "{report}"
    );
    assert_eq!(report["afterDispose"], 2, "{report}");
}
