#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use std::path::{Path, PathBuf};

struct CppStreams;

impl CppStreams {
    const FILES: [(&str, &str); 4] = [
        (
            "client/models/Tick.hpp",
            "#pragma once\n#include <string>\n\nstruct Tick {\n    std::string sku;\n    int level = 0;\n};\n",
        ),
        (
            "client/models/ChatMessage.hpp",
            "#pragma once\n#include <string>\n\nstruct ChatMessage {\n    std::string text;\n};\n",
        ),
        (
            "client/remotes/TicksClient.hpp",
            "#pragma once\n#include \"../models/Tick.hpp\"\n\nclass [[HTTPClient(\"/ticks\")]] TicksClient {\npublic:\n    [[StreamRequest]] void receive(const Tick& tick);\n};\n",
        ),
        (
            "client/remotes/ChatClient.hpp",
            "#pragma once\n#include \"../models/ChatMessage.hpp\"\n\nclass [[HTTPClient(\"/chat\")]] ChatClient {\npublic:\n    [[SocketRequest]] void receive(const ChatMessage& message);\n};\n",
        ),
    ];

    const SERVICE: (&str, &str, &str) = (
        "ticks",
        "TicksService",
        "#pragma once\n#include <string>\n#include \"../remotes/ChatClient.hpp\"\n#include \"../remotes/TicksClient.hpp\"\n\n\
         class [[Service]] TicksService {\npublic:\n    [[Needs]] TicksClient& ticks;\n    [[Needs]] ChatClient& chat;\n    int level = 0;\n    std::string said;\n\n    \
         [[OnEvent]] void heard(const Tick& tick) { level = tick.level; }\n\n    \
         [[OnEvent]] void answered(const ChatMessage& message) { said = message.text; }\n\n    \
         tachyon::Task<void> say(std::string text) { co_await chat.send(ChatMessage{text}); }\n};\n",
    );

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
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

    fn tools_available() -> bool {
        ["em++", "node"]
            .iter()
            .all(|program| crate::program_probe::ProgramProbe::answers(program, "--version"))
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
        let (instance, class, text) = Self::SERVICE;
        let path = Self::write(
            root,
            &format!("client/services/{class}.hpp"),
            text.as_bytes(),
        );
        let services = [(
            String::from(instance),
            String::from(class),
            WebLanguage::Cpp,
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
async fn a_cpp_service_receives_its_client_streams_as_events_and_sends_on_its_socket() {
    if !CppStreams::tools_available() {
        eprintln!("skipping: em++ (Emscripten) or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    CppStreams::publish(root.path()).await;
    let report = CppStreams::drive(root.path());
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
