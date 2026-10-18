#![allow(clippy::expect_used)]

use super::fsharp_tests::{fsharp_web_ready, run_driver, write};
use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;

struct DotnetHelpers;

impl DotnetHelpers {
    const CSHARP: [(&str, &str); 5] = [
        (
            "client/helpers/TaxRateHelper.cs",
            "[Helper]\npublic sealed class TaxRateHelper {\n    public int Asked = 0;\n\n    \
             public int Rate() {\n        Asked += 1;\n        return 2;\n    }\n}\n",
        ),
        (
            "client/helpers/TaxHelper.cs",
            "[Helper]\npublic sealed class TaxHelper {\n    [Needs]\n    private readonly TaxRateHelper _rates;\n\n    \
             public int AddTax(int subtotal) {\n        int tax = _rates.Rate();\n        return subtotal + tax;\n    }\n\n    \
             public int Asked() {\n        return _rates.Asked;\n    }\n}\n",
        ),
        (
            "client/helpers/RoundHelper.cs",
            "[Helper]\npublic sealed class RoundHelper {\n    public int Round(int value) {\n        return value / 10 * 10;\n    }\n}\n",
        ),
        (
            "client/services/CheckoutService.cs",
            "[Service]\npublic sealed class CheckoutService {\n    [Needs]\n    private readonly TaxHelper _tax;\n    [Needs]\n    private readonly RoundHelper _rounding;\n    public int Total = 0;\n\n    \
             public int Checkout() {\n        Total = _tax.AddTax(40);\n        return Total;\n    }\n\n    \
             public int Rounded() {\n        return _rounding.Round(Total);\n    }\n\n    \
             public int Asked() {\n        return _tax.Asked();\n    }\n}\n",
        ),
        (
            "client/services/QuoteService.cs",
            "[Service]\npublic sealed class QuoteService {\n    [Needs]\n    private readonly TaxHelper _tax;\n    [Needs]\n    private readonly RoundHelper _rounding;\n\n    \
             public int Quote(int subtotal) {\n        return _rounding.Round(_tax.AddTax(subtotal));\n    }\n\n    \
             public int Asked() {\n        return _tax.Asked();\n    }\n}\n",
        ),
    ];

    const FSHARP: [(&str, &str); 5] = [
        (
            "client/helpers/TaxRateHelper.fs",
            "[<Helper>]\ntype TaxRateHelper() =\n    member val Asked = 0 with get, set\n\n    \
             member this.Rate() : int =\n        this.Asked <- this.Asked + 1\n        2\n",
        ),
        (
            "client/helpers/TaxHelper.fs",
            "[<Helper>]\ntype TaxHelper() =\n    [<Needs>]\n    member val Rates : TaxRateHelper = Unchecked.defaultof<TaxRateHelper> with get, set\n\n    \
             member this.AddTax(subtotal: int) : int =\n        let tax = this.Rates.Rate()\n        subtotal + tax\n\n    \
             member this.Asked() : int = this.Rates.Asked\n",
        ),
        (
            "client/helpers/RoundHelper.fs",
            "[<Helper>]\ntype RoundHelper() =\n    member this.Round(value: int) : int = value / 10 * 10\n",
        ),
        (
            "client/services/CheckoutService.fs",
            "[<Service>]\ntype CheckoutService() =\n    [<Needs>]\n    member val Tax : TaxHelper = Unchecked.defaultof<TaxHelper> with get, set\n    \
             [<Needs>]\n    member val Rounding : RoundHelper = Unchecked.defaultof<RoundHelper> with get, set\n    \
             member val Total = 0 with get, set\n\n    \
             member this.Checkout() : int =\n        this.Total <- this.Tax.AddTax(40)\n        this.Total\n\n    \
             member this.Rounded() : int = this.Rounding.Round(this.Total)\n\n    \
             member this.Asked() : int = this.Tax.Asked()\n",
        ),
        (
            "client/services/QuoteService.fs",
            "[<Service>]\ntype QuoteService(tax: TaxHelper, rounding: RoundHelper) =\n    \
             member this.Quote(subtotal: int) : int = rounding.Round(tax.AddTax(subtotal))\n\n    \
             member this.Asked() : int = tax.Asked()\n",
        ),
    ];

    const DRIVER: &str = "const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const open = async (name) => {\n\
         \x20 const module = await import(`./${name}Service.js`)\n\
         \x20 const owner = new module.default({})\n\
         \x20 bus.attach(owner, module.__tachyon_event_plan__.types[0], { applicationEventPublisher: null })\n\
         \x20 await bus.initialize(owner)\n\
         \x20 return owner\n\
         }\n\
         const checkout = await open('Checkout'), quote = await open('Quote')\n\
         const answer = checkout.checkout()\n\
         const quoted = [quote.quote(100), quote.quote(7)]\n\
         const report = { answer, total: checkout.total, rounded: checkout.rounded(), quoted, asked: [checkout.asked(), quote.asked()], keys: Object.keys(checkout).sort() }\n\
         bus.dispose(checkout); bus.dispose(quote)\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";

    async fn publish(root: &Path, files: &[(&str, &str)]) {
        let mut services = Vec::new();
        for (relative, source) in files {
            let path = write(root, relative, source);
            let Some(class) = relative
                .strip_prefix("client/services/")
                .and_then(|name| name.split_once('.'))
                .map(|(class, _)| class)
            else {
                continue;
            };
            let instance = class.trim_end_matches("Service").to_ascii_lowercase();
            services.push((instance, String::from(class), WebLanguage::CSharp, path));
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
            write(root, &name, "");
            std::fs::write(root.join(name), bytes).expect("publish");
        }
        std::fs::write(
            root.join("application-events.mjs"),
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        )
        .expect("event runtime");
    }

    async fn report(files: &[(&str, &str)]) -> Option<serde_json::Value> {
        if !fsharp_web_ready() {
            eprintln!("skipping: the .NET wasm-tools workload or node is unavailable");
            return None;
        }
        let root = tempfile::tempdir().expect("project");
        Self::publish(root.path(), files).await;
        Some(run_driver(root.path(), Self::DRIVER))
    }

    fn check(report: &serde_json::Value) {
        assert_eq!(
            report,
            &serde_json::json!({
                "answer": 42,
                "total": 42,
                "rounded": 40,
                "quoted": [100, 0],
                "asked": [1, 2],
                "keys": ["total"],
            })
        );
    }
}

#[tokio::test]
async fn a_csharp_web_service_needs_a_helper_that_needs_another_helper() {
    if let Some(report) = DotnetHelpers::report(&DotnetHelpers::CSHARP).await {
        DotnetHelpers::check(&report);
    }
}

#[tokio::test]
async fn an_fsharp_web_service_needs_a_helper_that_needs_another_helper() {
    if let Some(report) = DotnetHelpers::report(&DotnetHelpers::FSHARP).await {
        DotnetHelpers::check(&report);
    }
}
