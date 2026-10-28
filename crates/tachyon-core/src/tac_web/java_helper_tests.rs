#![allow(clippy::expect_used)]

use super::java_tests::{java_web_ready, run_driver, write};
use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;

struct JavaHelpers;

impl JavaHelpers {
    const FILES: [(&str, &str); 5] = [
        (
            "client/delegates/TaxRateDelegate.java",
            "@Delegate\nclass TaxRateDelegate {\n    int asked = 0;\n\n    int rate() {\n        asked += 1;\n        return 2;\n    }\n}\n",
        ),
        (
            "client/delegates/TaxDelegate.java",
            "@Delegate\nclass TaxDelegate {\n    @Needs\n    private final TaxRateDelegate rates;\n\n    \
             int addTax(int subtotal) {\n        int tax = rates.rate();\n        return subtotal + tax;\n    }\n\n    \
             int asked() {\n        return rates.asked;\n    }\n}\n",
        ),
        (
            "client/delegates/RoundDelegate.java",
            "@Delegate\npublic class RoundDelegate {\n    public int round(int value) {\n        return value / 10 * 10;\n    }\n}\n",
        ),
        (
            "client/services/CheckoutService.java",
            "@Service\nclass CheckoutService {\n    @Needs\n    private final TaxDelegate helper;\n    @Needs\n    private final RoundDelegate rounding;\n    int total = 0;\n\n    \
             int checkout() {\n        total = helper.addTax(40);\n        return total;\n    }\n\n    \
             int rounded() {\n        return rounding.round(total);\n    }\n\n    \
             int asked() {\n        return helper.asked();\n    }\n}\n",
        ),
        (
            "client/services/QuoteService.java",
            "@Service\nclass QuoteService {\n    @Needs\n    private final TaxDelegate helper;\n\n    \
             int quote(int subtotal) {\n        return helper.addTax(subtotal);\n    }\n\n    \
             int asked() {\n        return helper.asked();\n    }\n}\n",
        ),
    ];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const { default: CheckoutService } = await import('./CheckoutService.js')\n\
         const { default: QuoteService } = await import('./QuoteService.js')\n\
         const checkout = new CheckoutService({}), quote = new QuoteService({})\n\
         checkout.__tachyonStarted(); quote.__tachyonStarted()\n\
         const answer = checkout.checkout()\n\
         const quoted = [quote.quote(100), quote.quote(7)]\n\
         const report = { answer, total: checkout.total, rounded: checkout.rounded(), quoted, asked: [checkout.asked(), quote.asked()], keys: Object.keys(checkout).sort() }\n\
         checkout.__tachyonDispose(); quote.__tachyonDispose()\n\
         console.log(JSON.stringify(report))\n\
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
        .unwrap_or_else(|failure| panic!("{failure}"));
        for (name, bytes) in built.files {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, bytes).expect("publish");
        }
    }
}

#[tokio::test]
async fn a_java_web_service_needs_a_helper_that_needs_another_helper() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    JavaHelpers::publish(root.path()).await;
    let report = run_driver(root.path(), JavaHelpers::DRIVER);
    assert_eq!(
        report,
        serde_json::json!({
            "answer": 42,
            "total": 42,
            "rounded": 40,
            "quoted": [102, 9],
            "asked": [1, 2],
            "keys": ["total"],
        })
    );
}
