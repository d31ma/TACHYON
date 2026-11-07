#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use std::path::Path;
use std::process::Command;

struct GoDelegateBaseFixtures;

impl GoDelegateBaseFixtures {
    const FILES: [(&str, &str); 3] = [
        (
            "client/services/TaxService.go",
            "// @Service\ntype TaxService struct {\n\tAsked int\n}\n\n\
             func (tax *TaxService) Rate(region string) int {\n\ttax.Asked++\n\treturn 2\n}\n",
        ),
        (
            "client/delegates/TaxDelegate.go",
            "// @Delegate\ntype TaxDelegate struct {\n\t// @Needs\n\ttaxService *TaxService\n}\n\n\
             func (delegate *TaxDelegate) Rate(region string) int {\n\treturn delegate.taxService.Rate(region) * 2\n}\n",
        ),
        (
            "client/services/QuoteService.go",
            "// @Service\ntype QuoteService struct {\n\t// @Needs\n\ttaxDelegate *TaxDelegate\n\tTotal       int\n}\n\n\
             func (quote *QuoteService) Quote() int {\n\tquote.Total = quote.taxDelegate.Rate(\"north\")\n\treturn quote.Total\n}\n",
        ),
    ];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const owners = {}\n\
         for (const name of ['Quote', 'Tax']) {\n\
         \x20 const module = await import(`./${name}Service.js`)\n\
         \x20 owners[name] = new module.default({})\n\
         \x20 bus.attach(owners[name], module.__tachyon_event_plan__.types[0], { applicationEventPublisher: null })\n\
         \x20 await bus.initialize(owners[name])\n\
         }\n\
         const { Quote: quote, Tax: tax } = owners\n\
         const quoted = [quote.quote(), quote.quote()]\n\
         const report = { quoted, asked: tax.asked, keys: Object.keys(quote) }\n\
         quote.__tachyonDispose()\n\
         tax.__tachyonDispose()\n\
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
            .unwrap_or_else(|failure| {
                panic!("a Go Delegate with a base service builds: {failure}")
            });
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
async fn a_go_delegate_takes_on_the_service_that_the_page_shares() {
    if [
        GoDelegateBaseFixtures::available("go"),
        GoDelegateBaseFixtures::available("node"),
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
    GoDelegateBaseFixtures::publish(root.path()).await;
    let report = GoDelegateBaseFixtures::drive(root.path());
    assert_eq!(report["quoted"], serde_json::json!([4, 4]), "{report}");
    assert_eq!(report["asked"], 2, "{report}");
    assert_eq!(report["keys"], serde_json::json!(["total"]), "{report}");
}
