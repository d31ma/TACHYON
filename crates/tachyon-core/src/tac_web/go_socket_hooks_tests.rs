#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use std::path::{Path, PathBuf};
use std::process::Command;

struct GoSocketHooks;

impl GoSocketHooks {
    const FILES: [(&str, &str); 2] = [
        (
            "client/models/ChatMessage.go",
            "type ChatMessage struct {\n\tText string `json:\"text\"`\n}\n",
        ),
        (
            "client/remotes/ChatClient.go",
            "// @HTTPClient(\"/chat\")\ntype ChatClient interface {\n\t// @SocketRequest\n\tReceive(message ChatMessage)\n}\n",
        ),
    ];

    const SERVICE: &str = "// @Service\ntype ChatService struct {\n\t// @Needs\n\tchat *ChatClient\n\tOpens    int\n\tReopens  int\n\tLastCode float64\n\tRetrying bool\n\tTicks    int\n}\n\n\
         // @OnEvent\nfunc (service *ChatService) Connected(event SocketOpened) error {\n\tservice.Opens++\n\tif event.Reconnected {\n\t\tservice.Reopens++\n\t}\n\treturn service.chat.Send(ChatMessage{Text: \"auth \" + event.Path})\n}\n\n\
         // @OnEvent\nfunc (service *ChatService) Disconnected(event SocketClosed) {\n\tservice.LastCode = event.Code\n\tservice.Retrying = event.Reconnecting\n}\n\n\
         // @Every(\"30s\")\nfunc (service *ChatService) Refresh() error {\n\tservice.Ticks++\n\treturn service.chat.Send(ChatMessage{Text: \"tick\"})\n}\n\n\
         func (service *ChatService) Say(text string) error {\n\treturn service.chat.Send(ChatMessage{Text: text})\n}\n";

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         globalThis.location = { href: 'http://127.0.0.1:4000/page' }\n\
         const shown = []\n\
         globalThis.document = { visibilityState: 'visible', querySelector: () => null, addEventListener: (name, listener) => { if (name === 'visibilitychange') shown.push(listener) } }\n\
         const intervals = [], cleared = []\n\
         globalThis.setInterval = (work, every) => { intervals.push({ work, every }); return intervals.length }\n\
         globalThis.clearInterval = (id) => { cleared.push(id) }\n\
         const sockets = []\n\
         globalThis.WebSocket = class { constructor(url) { this.url = String(url); this.readyState = 0; this.sent = []; sockets.push(this) } send(text) { this.sent.push(text) } close() { this.readyState = 3 } }\n\
         const pause = (time) => new Promise((resolve) => setTimeout(resolve, time))\n\
         const { default: ChatService, __tachyon_event_plan__: plan } = await import('./ChatService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         let failures = 0\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { failures += 1 } })\n\
         const chat = new ChatService({})\n\
         bus.attach(chat, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(chat)\n\
         const report = { every: intervals.map((interval) => interval.every) }\n\
         await chat.say('early')\n\
         const [first] = sockets\n\
         first.readyState = 1\n\
         first.onopen()\n\
         await pause(50)\n\
         report.firstSent = [...first.sent]\n\
         report.opens = chat.opens\n\
         first.readyState = 3\n\
         first.onclose({ code: 1006 })\n\
         await pause(50)\n\
         report.closed = { code: chat.lastCode, retrying: chat.retrying }\n\
         await pause(1100)\n\
         const second = sockets[1]\n\
         second.readyState = 1\n\
         second.onopen()\n\
         await pause(50)\n\
         report.secondSent = [...second.sent]\n\
         report.reopens = chat.reopens\n\
         await intervals[0].work()\n\
         report.ticksShown = chat.ticks\n\
         document.visibilityState = 'hidden'\n\
         await intervals[0].work()\n\
         report.ticksHidden = chat.ticks\n\
         document.visibilityState = 'visible'\n\
         for (const listener of shown) listener()\n\
         await pause(50)\n\
         report.ticksAgain = chat.ticks\n\
         report.lastSent = second.sent.at(-1)\n\
         bus.dispose(chat)\n\
         await pause(50)\n\
         report.cleared = cleared\n\
         report.failures = failures\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";

    fn tools_available() -> bool {
        [("go", "version"), ("node", "--version")]
            .iter()
            .all(|(tool, argument)| {
                Command::new(tool)
                    .arg(argument)
                    .output()
                    .is_ok_and(|output| output.status.success())
            })
    }

    fn write(root: &Path, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(&path, bytes).expect("file");
        path
    }

    async fn publish(root: &Path) {
        for (relative, text) in Self::FILES {
            Self::write(root, relative, text.as_bytes());
        }
        let path = Self::write(
            root,
            "client/services/ChatService.go",
            Self::SERVICE.as_bytes(),
        );
        let services = vec![(
            String::from("chat"),
            String::from("ChatService"),
            WebLanguage::Go,
            path,
        )];
        let profile = crate::BuildProfile::Development;
        let built = build_all(root, &services, &BuildJobs::new(1), profile)
            .await
            .unwrap_or_else(|failure| panic!("{failure}"));
        for (name, bytes) in built.files {
            Self::write(root, &name, &bytes);
        }
        Self::write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            )
            .as_bytes(),
        );
    }

    fn drive(root: &Path) -> serde_json::Value {
        Self::write(root, "drive.mjs", Self::DRIVER.as_bytes());
        let output = NodeDriver::output(NodeDriver::command().arg("drive.mjs").current_dir(root))
            .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.trim()).expect("report")
    }
}

#[tokio::test]
async fn a_go_service_hears_its_socket_open_and_close_and_runs_its_every_method_while_shown() {
    if !GoSocketHooks::tools_available() {
        assert!(
            std::env::var_os("CI").is_none(),
            "CI must provide go and node"
        );
        eprintln!("skipping: go or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    GoSocketHooks::publish(root.path()).await;
    let report = GoSocketHooks::drive(root.path());
    assert_eq!(report["every"], serde_json::json!([30000]), "{report}");
    assert_eq!(
        report["firstSent"],
        serde_json::json!(["{\"text\":\"auth /chat\"}", "{\"text\":\"early\"}"]),
        "the SocketOpened listener sends before the waiting messages: {report}"
    );
    assert_eq!(report["opens"], 1, "{report}");
    assert_eq!(
        report["closed"],
        serde_json::json!({"code": 1006, "retrying": true}),
        "{report}"
    );
    assert_eq!(
        report["secondSent"],
        serde_json::json!(["{\"text\":\"auth /chat\"}"]),
        "the service authenticates again after the reconnect: {report}"
    );
    assert_eq!(report["reopens"], 1, "{report}");
    assert_eq!(report["ticksShown"], 1, "{report}");
    assert_eq!(
        report["ticksHidden"], 1,
        "a hidden page skips the tick: {report}"
    );
    assert_eq!(
        report["ticksAgain"], 2,
        "the missed tick runs once the page is shown: {report}"
    );
    assert_eq!(report["lastSent"], "{\"text\":\"tick\"}", "{report}");
    assert_eq!(
        report["cleared"],
        serde_json::json!([1]),
        "unmounting stops the timer: {report}"
    );
    assert_eq!(report["failures"], 0, "{report}");
}
