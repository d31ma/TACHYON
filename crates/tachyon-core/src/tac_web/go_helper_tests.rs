#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;
use std::process::Command;

struct GoHelperFixtures;

impl GoHelperFixtures {
    const FILES: [(&str, &str); 3] = [
        (
            "client/helpers/StepHelper.go",
            "import \"strings\"\n\n// @Helper\ntype StepHelper struct {\n\ttaken int\n}\n\n\
             func (step *StepHelper) Next() int {\n\tstep.taken += len(strings.Repeat(\"a\", 2))\n\treturn step.taken\n}\n",
        ),
        (
            "client/services/TallyService.go",
            "// @Service\ntype TallyService struct {\n\t// @Needs\n\tstep  *StepHelper\n\tTotal int\n}\n\n\
             func (tally *TallyService) Bump() int {\n\ttally.Total = tally.step.Next()\n\treturn tally.Total\n}\n",
        ),
        (
            "client/services/CounterService.go",
            "// @Service\ntype CounterService struct {\n\tstep  *StepHelper\n\tCount int\n}\n\n\
             // @OnStart\nfunc (counter *CounterService) Start() {\n\tcounter.step = &StepHelper{taken: 10}\n}\n\n\
             func (counter *CounterService) Bump() int {\n\tcounter.Count = counter.step.Next()\n\treturn counter.Count\n}\n",
        ),
    ];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const owners = {}\n\
         for (const name of ['Tally', 'Counter']) {\n\
         \x20 const module = await import(`./${name}Service.js`)\n\
         \x20 owners[name] = new module.default({})\n\
         \x20 bus.attach(owners[name], module.__tachyon_event_plan__.types[0], { applicationEventPublisher: null })\n\
         \x20 await bus.initialize(owners[name])\n\
         }\n\
         const { Tally: tally, Counter: counter } = owners\n\
         const bumped = [tally.bump(), tally.bump(), counter.bump(), tally.bump()]\n\
         const report = { bumped, total: tally.total, count: counter.count, keys: Object.keys(tally) }\n\
         tally.__tachyonDispose()\n\
         counter.__tachyonDispose()\n\
         process.stdout.write(`${JSON.stringify(report)}\\n`)\n\
         process.exit(0)\n";

    fn available(program: &str) -> bool {
        Command::new(program)
            .arg(if program == "node" {
                "--version"
            } else {
                "version"
            })
            .output()
            .is_ok_and(|output| output.status.success())
    }

    async fn publish(root: &Path) {
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
        let profile = crate::BuildProfile::Development;
        let built = build_all(root, &services, &BuildJobs::new(1), profile)
            .await
            .unwrap_or_else(|failure| panic!("a Go service with a Go Helper builds: {failure}"));
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
    }

    fn drive(root: &Path) -> serde_json::Value {
        let output = Command::new("node")
            .arg("drive.mjs")
            .current_dir(root)
            .output()
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
async fn a_go_service_owns_the_go_helper_it_needs_or_creates() {
    if [
        GoHelperFixtures::available("go"),
        GoHelperFixtures::available("node"),
    ]
    .contains(&false)
    {
        assert!(
            std::env::var_os("CI").is_none(),
            "CI must provide go and node"
        );
        eprintln!("skipping: go or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    GoHelperFixtures::publish(root.path()).await;
    let report = GoHelperFixtures::drive(root.path());
    assert_eq!(
        report["bumped"],
        serde_json::json!([2, 4, 12, 6]),
        "{report}"
    );
    assert_eq!(report["total"], 6, "{report}");
    assert_eq!(report["count"], 12, "{report}");
    assert_eq!(report["keys"], serde_json::json!(["total"]), "{report}");
}
