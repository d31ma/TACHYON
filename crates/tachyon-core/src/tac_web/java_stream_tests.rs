#![allow(clippy::expect_used)]

use super::java_tests::{java_web_ready, run_driver, write};
use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;

struct JavaStreams;

impl JavaStreams {
    const FILES: [(&str, &str); 5] = [
        (
            "client/models/Tick.java",
            "record Tick(String sku, int level) {}\n",
        ),
        (
            "client/models/ChatMessage.java",
            "record ChatMessage(String text) {}\n",
        ),
        (
            "client/remotes/TicksClient.java",
            "import models.Tick;\n\n@HTTPClient(\"/ticks\")\ninterface TicksClient {\n    @StreamRequest\n    void receive(Tick tick);\n}\n",
        ),
        (
            "client/remotes/ChatClient.java",
            "import models.ChatMessage;\n\n@HTTPClient(\"/chat\")\ninterface ChatClient {\n    @SocketRequest\n    void receive(ChatMessage message);\n}\n",
        ),
        (
            "application-events.mjs",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        ),
    ];

    const SERVICES: [(&str, &str, &str); 2] = [
        (
            "ticks",
            "TicksService",
            "import models.ChatMessage;\nimport models.Tick;\n\n@Service\nclass TicksService {\n    @Needs\n    private final TicksClient ticks;\n    @Needs\n    private final ChatClient chat;\n    int level = 0;\n    String said = \"\";\n\n    \
             @OnEvent\n    void heard(Tick tick) {\n        level = tick.level();\n    }\n\n    \
             @OnEvent\n    void answered(ChatMessage message) {\n        said = message.text();\n    }\n\n    \
             void say(String text) {\n        chat.send(new ChatMessage(text));\n    }\n}\n",
        ),
        (
            "quiet",
            "QuietService",
            "@Service\nclass QuietService {\n    int count = 0;\n}\n",
        ),
    ];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         globalThis.location = { href: 'http://127.0.0.1:4000/page' }\n\
         const opened = []\n\
         globalThis.EventSource = class { constructor(url) { this.url = String(url); this.closed = false; opened.push(this) } addEventListener() {} close() { this.closed = true } }\n\
         globalThis.WebSocket = class { constructor(url) { this.url = String(url); this.readyState = 0; this.sent = []; this.closed = false; opened.push(this) } send(text) { this.sent.push(text) } close() { this.closed = true } }\n\
         const { default: TicksService, __tachyon_event_plan__: ticksPlan } = await import('./TicksService.js')\n\
         const { default: QuietService, __tachyon_event_plan__: quietPlan } = await import('./QuietService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         let failures = 0\n\
         const published = []\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { failures += 1 } })\n\
         bus.attach({ receive: (value) => published.push(value) }, { listeners: [{ method: 'receive', event: 'Tick' }] }, {})\n\
         const ticks = new TicksService({}), quiet = new QuietService({})\n\
         bus.attach(ticks, ticksPlan.types[0], { applicationEventPublisher: null })\n\
         bus.attach(quiet, quietPlan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(ticks)\n\
         await bus.initialize(quiet)\n\
         const stream = opened.find((source) => !('sent' in source)), socket = opened.find((source) => 'sent' in source)\n\
         const queued = [ticks.say('early')]\n\
         for (let index = 0; index < 15; index += 1) queued.push(ticks.say(`queued ${index}`))\n\
         let refused = ''\n\
         try { await ticks.say('overflow') } catch (error) { refused = error.message }\n\
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

    async fn publish(root: &Path) {
        for (relative, text) in Self::FILES {
            write(root, relative, text);
        }
        let services: Vec<_> = Self::SERVICES
            .iter()
            .map(|(instance, class, text)| {
                let path = write(root, &format!("client/services/{class}.java"), text);
                (
                    String::from(*instance),
                    String::from(*class),
                    WebLanguage::Java,
                    path,
                )
            })
            .collect();
        let profile = crate::BuildProfile::Development;
        let built = build_all(root, &services, &BuildJobs::new(1), profile)
            .await
            .unwrap_or_else(|failure| panic!("{failure}"));
        for (name, bytes) in built.files {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            std::fs::write(path, bytes).expect("publish");
        }
    }
}

#[tokio::test]
async fn a_java_web_service_receives_its_client_streams_as_events_and_sends_on_its_socket() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    JavaStreams::publish(root.path()).await;
    let report = run_driver(root.path(), JavaStreams::DRIVER);
    assert_eq!(
        report["urls"],
        serde_json::json!(["/ticks", "ws://127.0.0.1:4000/chat"]),
        "only the service that needs the clients opens them: {report}"
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
        report["refused"]
            .as_str()
            .is_some_and(|message| message.contains("HTTP_NETWORK")),
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
