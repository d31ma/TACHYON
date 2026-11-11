#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use std::path::{Path, PathBuf};

pub(crate) struct SocketHooks;

pub(crate) struct SocketHooksCase<'a> {
    pub(crate) language: WebLanguage,
    pub(crate) extension: &'a str,
    pub(crate) files: &'a [(&'a str, &'a str)],
    pub(crate) service: &'a str,
    pub(crate) sockets: bool,
}

impl SocketHooks {
    const FILE_FETCH: &'static str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n";

    const PAGE: &'static str = "globalThis.location = { href: 'http://127.0.0.1:4000/page' }\n\
         const shown = []\n\
         globalThis.document = { visibilityState: 'visible', querySelector: () => null, addEventListener: (name, listener) => { if (name === 'visibilitychange') shown.push(listener) } }\n\
         const intervals = [], cleared = []\n\
         globalThis.setInterval = (work, every) => { intervals.push({ work, every }); return intervals.length }\n\
         globalThis.clearInterval = (id) => { cleared.push(id) }\n\
         const sockets = []\n\
         globalThis.WebSocket = class { constructor(url) { this.url = String(url); this.readyState = 0; this.sent = []; sockets.push(this) } send(text) { this.sent.push(text) } close() { this.readyState = 3 } }\n\
         const pause = (time) => new Promise((resolve) => setTimeout(resolve, time))\n\
         const { default: ChatService, __tachyon_event_plan__: plan } = await import('./ChatService.js')\n\
         const { default: ApplicationEvents } = await import('./__EVENTS__')\n\
         let failures = 0\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { failures += 1 } })\n\
         const chat = new ChatService({})\n\
         bus.attach(chat, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(chat)\n\
         const report = { every: intervals.map((interval) => interval.every) }\n";

    const SOCKET_STEPS: &'static str = "const [first] = sockets\n\
         first.readyState = 1\n\
         first.onopen()\n\
         await pause(50)\n\
         report.opens = chat.opens\n\
         first.readyState = 3\n\
         first.onclose({ code: 1006 })\n\
         await pause(50)\n\
         report.closed = { code: chat.code, retrying: chat.retrying }\n\
         await pause(1100)\n\
         const second = sockets[1]\n\
         second.readyState = 1\n\
         second.onopen()\n\
         await pause(50)\n\
         report.reopens = chat.reopens\n";

    const TIMER_STEPS: &'static str = "await intervals[0].work()\n\
         report.ticksShown = chat.ticks\n\
         document.visibilityState = 'hidden'\n\
         await intervals[0].work()\n\
         report.ticksHidden = chat.ticks\n\
         document.visibilityState = 'visible'\n\
         for (const listener of shown) listener()\n\
         await pause(50)\n\
         report.ticksAgain = chat.ticks\n\
         bus.dispose(chat)\n\
         await pause(50)\n\
         report.cleared = cleared\n\
         report.failures = failures\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";

    pub(crate) async fn run(root: &Path, case: &SocketHooksCase<'_>) -> serde_json::Value {
        Self::publish(root, case).await;
        Self::drive(root, case.sockets)
    }

    pub(crate) fn drive(root: &Path, sockets: bool) -> serde_json::Value {
        Self::write_events(root);
        let driver = Self::driver(sockets, "application-events.mjs");
        Self::write(root, "drive.mjs", driver.as_bytes());
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

    pub(crate) fn driver(sockets: bool, events_module: &str) -> String {
        format!("{}{}", Self::FILE_FETCH, Self::body(sockets, events_module))
    }

    pub(crate) fn body(sockets: bool, events_module: &str) -> String {
        let steps = ["", Self::SOCKET_STEPS];
        format!(
            "{}{}{}",
            Self::PAGE.replace("__EVENTS__", events_module),
            steps[usize::from(sockets)],
            Self::TIMER_STEPS
        )
    }

    pub(crate) async fn prepare(root: &Path, case: &SocketHooksCase<'_>) {
        Self::publish(root, case).await;
        Self::write_events(root);
    }

    pub(crate) fn check(report: &serde_json::Value, sockets: bool) {
        assert_eq!(report["every"], serde_json::json!([30000]), "{report}");
        assert_eq!(report["ticksShown"], 1, "{report}");
        assert_eq!(
            report["ticksHidden"], 1,
            "a hidden page skips the tick: {report}"
        );
        assert_eq!(
            report["ticksAgain"], 2,
            "the missed tick runs once the page is shown: {report}"
        );
        assert_eq!(report["cleared"], serde_json::json!([1]), "{report}");
        assert_eq!(report["failures"], 0, "{report}");
        let expected = [
            None,
            Some(serde_json::json!({"code": 1006, "retrying": true})),
        ];
        assert_eq!(
            report.get("closed").cloned(),
            expected[usize::from(sockets)],
            "{report}"
        );
        assert_eq!(
            report.get("opens").cloned(),
            sockets.then(|| serde_json::json!(1))
        );
        assert_eq!(
            report.get("reopens").cloned(),
            sockets.then(|| serde_json::json!(1))
        );
    }

    async fn publish(root: &Path, case: &SocketHooksCase<'_>) {
        for (relative, text) in case.files {
            Self::write(root, relative, text.as_bytes());
        }
        let path = Self::write(
            root,
            &format!("client/services/ChatService.{}", case.extension),
            case.service.as_bytes(),
        );
        let services = vec![(
            String::from("chat"),
            String::from("ChatService"),
            case.language,
            path,
        )];
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Development,
        )
        .await
        .unwrap_or_else(|failure| panic!("{failure}"));
        for (name, bytes) in built.files {
            Self::write(root, &name, &bytes);
        }
    }

    fn write_events(root: &Path) {
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

    pub(crate) fn write(root: &Path, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(&path, bytes).expect("file");
        path
    }
}
