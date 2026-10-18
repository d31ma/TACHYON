#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use std::path::Path;
use std::process::Command;

struct GoNeedsFixtures;

impl GoNeedsFixtures {
    const FILES: [(&str, &str); 4] = [
        (
            "client/models/CounterBumped.go",
            "type CounterBumped struct {\n\tCount int `json:\"count\"`\n}\n",
        ),
        (
            "client/services/CounterService.go",
            "import \"fmt\"\n\n// @Service\ntype CounterService struct {\n\tCount     int\n\tpublisher ApplicationEventPublisher\n}\n\n\
             // @OnStart\nfunc (counter *CounterService) Start() {\n\tfmt.Println(\"needs: start counter\")\n}\n\n\
             // @OnStop\nfunc (counter *CounterService) Stop() {\n\tfmt.Println(\"needs: stop counter\")\n}\n\n\
             func (counter *CounterService) Increment() int {\n\tcounter.Count++\n\tcounter.publisher.PublishEvent(CounterBumped{Count: counter.Count}, false)\n\treturn counter.Count\n}\n",
        ),
        (
            "client/services/TallyService.go",
            "import \"fmt\"\n\n// @Service\ntype TallyService struct {\n\t// @Needs\n\tcounter *CounterService\n\tTotal   int\n}\n\n\
             // @OnStart\nfunc (tally *TallyService) Start() {\n\tfmt.Println(\"needs: start tally after\", tally.counter.Count)\n}\n\n\
             // @OnStop\nfunc (tally *TallyService) Stop() {\n\tfmt.Println(\"needs: stop tally at\", tally.counter.Count)\n}\n\n\
             func (tally *TallyService) Bump() int {\n\ttally.Total = tally.counter.Increment()\n\treturn tally.Total\n}\n",
        ),
        (
            "client/services/WatchService.go",
            "// @Service\ntype WatchService struct {\n\tSeen int\n}\n\n\
             // @OnEvent\nfunc (watch *WatchService) Saw(value CounterBumped) {\n\twatch.Seen = value.Count\n}\n",
        ),
    ];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const lines = []\n\
         for (const level of ['log', 'info', 'warn', 'error']) console[level] = (...values) => lines.push(values.join(' '))\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const modules = {}\n\
         for (const name of ['Counter', 'Tally', 'Watch']) modules[name] = await import(`./${name}Service.js`)\n\
         const events = []\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         bus.attach({ receive: (value) => events.push(value.count) }, { listeners: [{ method: 'receive', event: 'CounterBumped', order: -100 }] }, {})\n\
         const [created, disposed] = process.argv.slice(2)\n\
         const owners = {}\n\
         for (const name of [...created.split(','), 'Watch']) {\n\
         \x20 const module = modules[name]\n\
         \x20 owners[name] = new module.default({})\n\
         \x20 bus.attach(owners[name], module.__tachyon_event_plan__.types[0], { applicationEventPublisher: null })\n\
         \x20 await bus.initialize(owners[name])\n\
         }\n\
         const { Counter: counter, Tally: tally, Watch: watch } = owners\n\
         const bumped = [tally.bump(), tally.bump(), counter.increment()]\n\
         const seen = { count: counter.count, total: tally.total, watched: watch.seen }\n\
         const [first, second] = disposed.split(',')\n\
         owners[first].__tachyonDispose()\n\
         let later = null\n\
         try { later = first === 'Counter' ? tally.bump() : counter.increment() } catch (error) { later = String(error?.message ?? error) }\n\
         owners[second].__tachyonDispose()\n\
         const again = new modules.Tally.default({})\n\
         bus.attach(again, modules.Tally.__tachyon_event_plan__.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(again)\n\
         const resumed = again.bump()\n\
         again.__tachyonDispose()\n\
         watch.__tachyonDispose()\n\
         process.stdout.write(`${JSON.stringify({ bumped, seen, later, resumed, events, keys: Object.keys(tally), lines: lines.filter((line) => line.startsWith('needs: ')) })}\\n`)\n\
         process.exit(0)\n";

    fn available(program: &str, argument: &str) -> bool {
        Command::new(program)
            .arg(argument)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    async fn publish(root: &Path, profile: crate::BuildProfile) -> Vec<String> {
        let mut services = Vec::new();
        for (relative, source) in Self::FILES {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(&path, source).expect("source");
            let Some(class) = relative
                .strip_prefix("client/services/")
                .and_then(|name| name.strip_suffix(".go"))
            else {
                continue;
            };
            let instance = class.trim_end_matches("Service").to_ascii_lowercase();
            services.push((instance, String::from(class), WebLanguage::Go, path));
        }
        let built = build_all(root, &services, &BuildJobs::new(1), profile)
            .await
            .expect("a Go service that needs another Go service builds");
        for (name, bytes) in built.files {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, bytes).expect("publish");
        }
        std::fs::write(
            root.join("application-events.mjs"),
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        )
        .expect("event runtime");
        std::fs::write(root.join("drive.mjs"), Self::DRIVER).expect("driver");
        built.notes
    }

    fn drive(root: &Path, created: &str, disposed: &str) -> serde_json::Value {
        let output = NodeDriver::output(
            NodeDriver::command()
                .args(["drive.mjs", created, disposed])
                .current_dir(root),
        )
        .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.trim()).expect("report")
    }

    fn assert_shared(root: &Path) {
        for (created, disposed, stopped) in [
            ("Tally,Counter", "Counter,Tally", 4),
            ("Counter,Tally", "Tally,Counter", 3),
        ] {
            let report = Self::drive(root, created, disposed);
            assert_eq!(report["bumped"], serde_json::json!([1, 2, 3]), "{report}");
            assert_eq!(
                report["seen"],
                serde_json::json!({"count": 3, "total": 2, "watched": 3}),
                "{report}"
            );
            assert_eq!(report["later"], 4, "{report}");
            assert_eq!(report["resumed"], 5, "{report}");
            assert_eq!(
                report["events"],
                serde_json::json!([1, 2, 3, 4, 5]),
                "{report}"
            );
            assert_eq!(report["keys"], serde_json::json!(["total"]), "{report}");
            assert_eq!(
                report["lines"],
                serde_json::json!([
                    "needs: start counter",
                    "needs: start tally after 0",
                    format!("needs: stop tally at {stopped}"),
                    "needs: start tally after 4",
                    "needs: stop tally at 5",
                    "needs: stop counter",
                ]),
                "{created} then {disposed}: {report}"
            );
        }
    }
}

#[tokio::test]
async fn a_go_service_shares_the_go_service_it_needs_with_the_page() {
    let tools = [
        GoNeedsFixtures::available("go", "version"),
        GoNeedsFixtures::available("node", "--version"),
    ];
    if tools.contains(&false) {
        assert!(
            std::env::var_os("CI").is_none(),
            "CI must provide go and node"
        );
        eprintln!("skipping: go or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let notes = GoNeedsFixtures::publish(root.path(), crate::BuildProfile::Development).await;
    assert!(notes.is_empty(), "{notes:?}");
    GoNeedsFixtures::assert_shared(root.path());
}

#[tokio::test]
async fn a_tinygo_module_shares_the_go_service_it_needs_with_the_page() {
    let tools = [
        GoNeedsFixtures::available("go", "version"),
        GoNeedsFixtures::available("tinygo", "version"),
        GoNeedsFixtures::available("node", "--version"),
    ];
    if tools.contains(&false) {
        eprintln!("skipping: go, tinygo or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    std::fs::create_dir_all(root.path().join("client")).expect("client");
    std::fs::write(
        root.path().join("client/manifest.json"),
        "{\"tac\":{\"go\":{\"compiler\":\"tinygo\"}}}",
    )
    .expect("manifest");
    let notes = GoNeedsFixtures::publish(root.path(), crate::BuildProfile::Release).await;
    assert!(notes[0].starts_with("Go: TinyGo"), "{notes:?}");
    GoNeedsFixtures::assert_shared(root.path());
}
