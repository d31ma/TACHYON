#![allow(clippy::expect_used)]

use super::java_tests::{java_web_ready, run_driver, write};
use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;

struct JavaDelegateBaseFixtures;

impl JavaDelegateBaseFixtures {
    const FILES: [(&str, &str); 3] = [
        (
            "client/services/TaxService.java",
            "@Service\nclass TaxService {\n    int asked = 0;\n\n    \
             int rate(String region) {\n        asked += 1;\n        return 2;\n    }\n}\n",
        ),
        (
            "client/delegates/TaxDelegate.java",
            "@Delegate\nclass TaxDelegate {\n    @Needs\n    private final TaxService taxService;\n\n    \
             int rate(String region) {\n        return taxService.rate(region) * 2;\n    }\n}\n",
        ),
        (
            "client/services/QuoteService.java",
            "@Service\nclass QuoteService {\n    @Needs\n    private final TaxDelegate taxDelegate;\n    int total = 0;\n\n    \
             int quote() {\n        total = taxDelegate.rate(\"north\");\n        return total;\n    }\n}\n",
        ),
    ];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const open = async (name) => {\n\
         \x20 const module = await import(`./${name}Service.js`)\n\
         \x20 const owner = new module.default({})\n\
         \x20 const plan = module.__tachyon_event_plan__?.types?.[0]\n\
         \x20 if (plan) { bus.attach(owner, plan, { applicationEventPublisher: null }); await bus.initialize(owner) } else owner.__tachyonStarted()\n\
         \x20 return owner\n\
         }\n\
         const quote = await open('Quote'), tax = await open('Tax')\n\
         const quoted = [quote.quote(), quote.quote()]\n\
         const report = { quoted, asked: tax.asked, keys: Object.keys(quote) }\n\
         quote.__tachyonDispose()\n\
         tax.__tachyonDispose()\n\
         process.stdout.write(`${JSON.stringify(report)}\\n`)\n\
         process.exit(0)\n";

    async fn publish(root: &Path) {
        let mut services = Vec::new();
        for (relative, source) in Self::FILES {
            let path = write(root, relative, source);
            let Some(class) = relative
                .strip_prefix("client/services/")
                .and_then(|name| name.strip_suffix(".java"))
            else {
                continue;
            };
            let instance = class.trim_end_matches("Service").to_ascii_lowercase();
            services.push((instance, String::from(class), WebLanguage::Java, path));
        }
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Development,
        )
        .await
        .expect("a Java Delegate with a base service builds");
        for (name, bytes) in built.files {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, bytes).expect("publish");
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
async fn a_java_delegate_takes_on_the_service_that_the_page_shares() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    JavaDelegateBaseFixtures::publish(root.path()).await;
    let report = run_driver(root.path(), JavaDelegateBaseFixtures::DRIVER);
    assert_eq!(report["quoted"], serde_json::json!([4, 4]), "{report}");
    assert_eq!(report["asked"], 2, "{report}");
    assert_eq!(report["keys"], serde_json::json!(["total"]), "{report}");
}
