use super::PythonWebServices;
use super::page_tests::PythonPage;
use std::path::Path;

struct StreamProject;

impl StreamProject {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/models/Tick.py",
            "from dataclasses import dataclass\n\n@dataclass\nclass Tick:\n    level: int\n",
        ),
        (
            "client/models/ChatMessage.py",
            "from dataclasses import dataclass\n\n@dataclass\nclass ChatMessage:\n    text: str\n",
        ),
        (
            "client/remotes/TicksClient.py",
            "from typing import Protocol\nfrom models import Tick\n\n@http_client(\"/ticks\")\nclass TicksClient(Protocol):\n    @stream_request\n    def receive(self, tick: Tick) -> None: ...\n",
        ),
        (
            "client/remotes/ChatClient.py",
            "from typing import Protocol\nfrom models import ChatMessage\n\n@http_client(\"/chat\")\nclass ChatClient(Protocol):\n    @socket_request\n    def receive(self, message: ChatMessage) -> None: ...\n",
        ),
    ];

    const SERVICE: &'static str = "from typing import Annotated\nfrom models import ChatMessage, Tick\n\n@service\nclass TicksService:\n    ticks: Annotated[TicksClient, needs]\n    chat: Annotated[ChatClient, needs]\n    level: int = 0\n    said: str = ''\n\n    @on_event\n    def heard(self, tick: Tick) -> None:\n        self.level = tick.level * 10 + 1\n\n    @on_event\n    def answered(self, message: ChatMessage) -> None:\n        self.said = message.text\n\n    async def say(self, text: str) -> None:\n        await self.chat.send(ChatMessage(text))\n\n    async def say_too_much(self) -> str:\n        try:\n            await self.chat.send(ChatMessage('x' * 70000))\n        except BehaviorError as error:\n            return error.code\n        return ''\n";

    const QUIET: &'static str = "from models import Tick\n\n@service\nclass TicksService:\n    level: int = 0\n\n    @on_event\n    def heard(self, tick: Tick) -> None:\n        self.level = tick.level\n";

    const DRIVER: &'static str = "globalThis.location = { href: 'http://127.0.0.1:4000/page' }\n\
         const opened = []\n\
         globalThis.EventSource = class { constructor(url) { this.url = String(url); this.closed = false; opened.push(this) } addEventListener() {} close() { this.closed = true } }\n\
         globalThis.WebSocket = class { constructor(url) { this.url = String(url); this.readyState = 0; this.sent = []; this.closed = false; opened.push(this) } send(text) { this.sent.push(text) } close() { this.closed = true } }\n\
         const { default: TicksService, __tachyon_event_plan__: plan } = await import('./TicksService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         let failures = 0\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { failures += 1 } })\n\
         const ticks = new TicksService({})\n\
         bus.attach(ticks, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(ticks)\n\
         const stream = opened.find((source) => !('sent' in source)), socket = opened.find((source) => 'sent' in source)\n\
         const queued = [ticks.say('early')]\n\
         for (let index = 0; index < 15; index += 1) queued.push(ticks.say(`queued ${index}`))\n\
         let refused = ''\n\
         try { await ticks.say('overflow') } catch (error) { refused = error.code }\n\
         socket.readyState = 1\n\
         socket.onopen()\n\
         await Promise.all(queued)\n\
         stream.onmessage({ data: JSON.stringify({ level: 3 }), lastEventId: '' })\n\
         stream.onmessage({ data: JSON.stringify({ level: 'high' }), lastEventId: '' })\n\
         socket.onmessage({ data: JSON.stringify({ text: 'hello' }) })\n\
         await ticks.say('late')\n\
         const tooMuch = await ticks.sayTooMuch()\n\
         const report = { urls: opened.map((source) => source.url).sort(), level: ticks.level, said: ticks.said, failures, sent: socket.sent, tooMuch, refused }\n\
         bus.dispose(ticks)\n\
         await new Promise((resolve) => setTimeout(resolve, 50))\n\
         report.closed = opened.map((source) => source.closed)\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";

    const SAMPLES: [[(&'static str, &'static str); 3]; 2] = [
        [
            (
                "client/models/Tick.py",
                include_str!("../../../../website/content/features/realtime/client/models/Tick.py"),
            ),
            (
                "client/remotes/TicksClient.py",
                include_str!(
                    "../../../../website/content/features/realtime/client/remotes/TicksClient.py"
                ),
            ),
            (
                "client/services/StreamService.py",
                include_str!(
                    "../../../../website/content/features/realtime/client/services/StreamService.py"
                ),
            ),
        ],
        [
            (
                "client/models/ChatMessage.py",
                include_str!(
                    "../../../../website/content/features/sockets/client/models/ChatMessage.py"
                ),
            ),
            (
                "client/remotes/ChatClient.py",
                include_str!(
                    "../../../../website/content/features/sockets/client/remotes/ChatClient.py"
                ),
            ),
            (
                "client/services/ChatService.py",
                include_str!(
                    "../../../../website/content/features/sockets/client/services/ChatService.py"
                ),
            ),
        ],
    ];

    fn write_files(root: &Path) {
        for (relative, text) in Self::FILES {
            PythonPage::write(root, relative, text);
        }
    }

    fn program(root: &Path, service: &str) -> Result<String, crate::Failure> {
        Self::write_files(root);
        let path = PythonPage::write(root, "client/services/TicksService.py", service);
        PythonWebServices::program(
            root,
            &[(String::from("ticks"), String::from("TicksService"), path)],
        )
    }

    async fn publish(root: &Path) {
        Self::write_files(root);
        PythonPage::publish(root, &[("ticks", "TicksService", Self::SERVICE)]).await;
        PythonPage::write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        );
    }
}

#[test]
fn a_python_service_describes_the_streams_of_the_clients_it_needs() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    let program = StreamProject::program(root.path(), StreamProject::SERVICE)
        .unwrap_or_else(|failure| panic!("program: {failure}"));
    for expected in [
        "\\\"client\\\":\\\"ChatClient\\\",\\\"event\\\":\\\"ChatMessage\\\"",
        "\\\"path\\\":\\\"/chat\\\",\\\"socket\\\":true}",
        "\\\"client\\\":\\\"TicksClient\\\",\\\"event\\\":\\\"Tick\\\"",
        "\\\"path\\\":\\\"/ticks\\\",\\\"socket\\\":false}",
        "await _BehaviorBrowserSocket.send(\"ChatClient\", message)",
        "class _BehaviorBrowserSocket:",
    ] {
        assert!(program.contains(expected), "{expected}: {program}");
    }
    assert!(!program.contains("Behaviors.create(ChatSetup"), "{program}");
    assert!(
        !program.contains("Behaviors.create(TicksSetup"),
        "{program}"
    );
}

#[test]
fn a_service_that_does_not_need_a_stream_client_holds_no_stream() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    let program = StreamProject::program(root.path(), StreamProject::QUIET)
        .unwrap_or_else(|failure| panic!("program: {failure}"));
    assert!(program.contains("\\\"streams\\\":[]"), "{program}");
    assert!(
        !program.contains("\\\"path\\\":\\\"/ticks\\\""),
        "{program}"
    );
}

#[tokio::test]
async fn a_python_service_receives_streams_and_sends_on_its_socket() {
    if let Some(reason) = PythonPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    StreamProject::publish(root.path()).await;
    let report = PythonPage::run(root.path(), StreamProject::DRIVER);
    assert_eq!(
        report["urls"],
        serde_json::json!(["/ticks", "ws://127.0.0.1:4000/chat"]),
        "{report}"
    );
    assert_eq!(report["level"], 31, "{report}");
    assert_eq!(report["said"], "hello", "{report}");
    assert_eq!(report["failures"], 1, "{report}");
    assert_eq!(report["refused"], "HTTP_NETWORK", "{report}");
    let sent = report["sent"]
        .as_array()
        .unwrap_or_else(|| panic!("sent: {report}"));
    assert_eq!(sent.len(), 17, "{report}");
    assert_eq!(sent[0], "{\"text\":\"early\"}", "{report}");
    assert_eq!(sent[15], "{\"text\":\"queued 14\"}", "{report}");
    assert_eq!(sent[16], "{\"text\":\"late\"}", "{report}");
    assert_eq!(report["tooMuch"], "HTTP_BODY", "{report}");
    assert_eq!(
        report["closed"],
        serde_json::json!([true, true]),
        "{report}"
    );
}

#[test]
fn the_website_python_stream_and_socket_samples_build() {
    for (sample, (instance, class, client)) in StreamProject::SAMPLES.iter().zip([
        ("stream", "StreamService", "TicksClient"),
        ("chat", "ChatService", "ChatClient"),
    ]) {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
        for (relative, text) in sample {
            PythonPage::write(root.path(), relative, text);
        }
        let path = root.path().join(format!("client/services/{class}.py"));
        let program = PythonWebServices::program(
            root.path(),
            &[(String::from(instance), String::from(class), path)],
        )
        .unwrap_or_else(|failure| panic!("{class}: {failure}"));
        let held = format!("\\\"client\\\":\\\"{client}\\\"");
        assert!(program.contains(&held), "{held}: {program}");
    }
}
