#![allow(clippy::expect_used)]

use super::fsharp_tests::{run_driver, write};
use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use std::path::Path;
use std::process::Command;

struct CppStopPage;

impl CppStopPage {
    const SERVICE: &'static str = "#pragma once\n#include <cstdio>\n#include <string>\n\n\
         class [[Service]] WelcomeService {\npublic:\n    int count = 0;\n\n    \
         [[OnStart]] void start() { std::puts(\"hook: start welcome\"); }\n\n    \
         [[OnStop]] void stop() { std::printf(\"hook: stop welcome at %d\\n\", count); }\n\n    \
         int bump() { count += 1; return count; }\n};\n";

    const DRIVER: &'static str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const lines = []\n\
         for (const level of ['log', 'info', 'warn', 'error']) console[level] = (...values) => lines.push(values.join(' '))\n\
         const { default: WelcomeService, __tachyon_event_plan__: plan } = await import('./WelcomeService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const welcome = new WelcomeService({})\n\
         bus.attach(welcome, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(welcome)\n\
         const bumped = [welcome.bump(), welcome.bump()]\n\
         const running = lines.filter((line) => line.startsWith('hook: '))\n\
         bus.dispose(welcome)\n\
         await new Promise((resolve) => setTimeout(resolve, 50))\n\
         process.stdout.write(`${JSON.stringify({ bumped, running, lines: lines.filter((line) => line.startsWith('hook: ')) })}\\n`)\n\
         process.exit(0)\n";

    fn ready() -> bool {
        let compiler = Command::new("em++")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success());
        [compiler, NodeDriver::available()] == [true, true]
    }

    async fn publish(root: &Path) {
        let path = write(root, "client/services/WelcomeService.hpp", Self::SERVICE);
        let services = [(
            String::from("welcome"),
            String::from("WelcomeService"),
            WebLanguage::Cpp,
            path,
        )];
        let profile = crate::BuildProfile::Development;
        let built = build_all(root, &services, &BuildJobs::new(1), profile)
            .await
            .unwrap_or_else(|failure| panic!("{failure}"));
        for (name, bytes) in built.files {
            write(root, &name, "");
            std::fs::write(root.join(name), bytes).expect("publish");
        }
        write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        );
    }
}

#[tokio::test]
async fn a_cpp_service_runs_its_stop_hook_when_its_owner_is_disposed() {
    if !CppStopPage::ready() {
        eprintln!("skipping: em++ (Emscripten) or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    CppStopPage::publish(root.path()).await;
    let report = run_driver(root.path(), CppStopPage::DRIVER);
    assert_eq!(report["bumped"], serde_json::json!([1, 2]), "{report}");
    assert_eq!(
        report["running"],
        serde_json::json!(["hook: start welcome"]),
        "{report}"
    );
    assert_eq!(
        report["lines"],
        serde_json::json!(["hook: start welcome", "hook: stop welcome at 2"]),
        "{report}"
    );
}
