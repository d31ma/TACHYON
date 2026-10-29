#![allow(clippy::expect_used)]

use super::fsharp_tests::{fsharp_web_ready, run_driver, write};
use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;

struct FsharpDelegateBaseFixtures;

impl FsharpDelegateBaseFixtures {
    const FILES: [(&str, &str); 3] = [
        (
            "client/services/TaxService.fs",
            "[<Service>]\ntype TaxService() =\n    member val Asked = 0 with get, set\n\n    \
             member this.Rate(region: string) : int =\n        this.Asked <- this.Asked + 1\n        2\n",
        ),
        (
            "client/delegates/TaxDelegate.fs",
            "[<Delegate>]\ntype TaxDelegate() =\n    [<Needs>]\n    member val TaxService : TaxService = Unchecked.defaultof<TaxService> with get, set\n\n    \
             member this.Rate(region: string) : int =\n        this.TaxService.Rate(region) * 2\n",
        ),
        (
            "client/services/QuoteService.fs",
            "[<Service>]\ntype QuoteService() =\n    [<Needs>]\n    member val TaxDelegate : TaxDelegate = Unchecked.defaultof<TaxDelegate> with get, set\n    \
             member val Total = 0 with get, set\n\n    \
             member this.Quote() : int =\n        this.Total <- this.TaxDelegate.Rate(\"north\")\n        this.Total\n",
        ),
    ];

    const OVERLOAD_FILES: [(&str, &str); 3] = [
        (
            "client/services/TaxService.fs",
            "[<Service>]\ntype TaxService() =\n    member val Asked = 0 with get, set\n\n    \
             member this.Rate(region: string) : int =\n        this.Asked <- this.Asked + 1\n        2\n\n    \
             member this.Total(count: int) : int =\n        count * 10\n",
        ),
        (
            "client/delegates/TaxDelegate.fs",
            "[<Delegate>]\ntype TaxDelegate() =\n    [<Needs>]\n    member val TaxService : TaxService = Unchecked.defaultof<TaxService> with get, set\n\n    \
             member this.Rate(region: string) : int =\n        this.TaxService.Rate(region) * 2\n\n    \
             member this.Total(count: int, extra: int) : int =\n        count * 10 + extra\n",
        ),
        (
            "client/services/QuoteService.fs",
            "[<Service>]\ntype QuoteService() =\n    [<Needs>]\n    member val TaxDelegate : TaxDelegate = Unchecked.defaultof<TaxDelegate> with get, set\n\n    \
             member this.Rate() : int =\n        this.TaxDelegate.Rate(\"north\")\n\n    \
             member this.Total() : int =\n        this.TaxDelegate.Total(3)\n\n    \
             member this.Extra() : int =\n        this.TaxDelegate.Total(3, 4)\n",
        ),
    ];

    const OVERLOAD_DRIVER: &str = "const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const open = async (name) => {\n\
         \x20 const module = await import(`./${name}Service.js`)\n\
         \x20 const owner = new module.default({})\n\
         \x20 bus.attach(owner, module.__tachyon_event_plan__.types[0], { applicationEventPublisher: null })\n\
         \x20 await bus.initialize(owner)\n\
         \x20 return owner\n\
         }\n\
         const quote = await open('Quote'), tax = await open('Tax')\n\
         const quoted = { rate: quote.rate(), total: quote.total(), extra: quote.extra() }\n\
         const report = { quoted, asked: tax.asked }\n\
         bus.dispose(quote); bus.dispose(tax)\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";

    const DRIVER: &str = "const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const open = async (name) => {\n\
         \x20 const module = await import(`./${name}Service.js`)\n\
         \x20 const owner = new module.default({})\n\
         \x20 bus.attach(owner, module.__tachyon_event_plan__.types[0], { applicationEventPublisher: null })\n\
         \x20 await bus.initialize(owner)\n\
         \x20 return owner\n\
         }\n\
         const quote = await open('Quote'), tax = await open('Tax')\n\
         const quoted = [quote.quote(), quote.quote()]\n\
         const report = { quoted, asked: tax.asked, keys: Object.keys(quote) }\n\
         bus.dispose(quote); bus.dispose(tax)\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";

    async fn publish(root: &Path, files: &[(&str, &str)]) {
        let mut services = Vec::new();
        for (relative, source) in crate::delegate_forwards::DelegateForwards::staged_project(files)
        {
            let path = write(root, relative, &source);
            let Some(class) = relative
                .strip_prefix("client/services/")
                .and_then(|name| name.strip_suffix(".fs"))
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
        .unwrap_or_else(|failure| panic!("an F# Delegate with a base service builds: {failure}"));
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
async fn an_fsharp_delegate_takes_on_the_service_that_the_page_shares() {
    if !fsharp_web_ready() {
        eprintln!("skipping: the .NET wasm-tools workload or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    FsharpDelegateBaseFixtures::publish(root.path(), &FsharpDelegateBaseFixtures::FILES).await;
    let report = run_driver(root.path(), FsharpDelegateBaseFixtures::DRIVER);
    assert_eq!(report["quoted"], serde_json::json!([4, 4]), "{report}");
    assert_eq!(report["asked"], 2, "{report}");
    assert_eq!(report["keys"], serde_json::json!(["total"]), "{report}");
}

#[tokio::test]
async fn an_fsharp_delegate_overloads_a_base_method_by_signature() {
    if !fsharp_web_ready() {
        eprintln!("skipping: the .NET wasm-tools workload or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    FsharpDelegateBaseFixtures::publish(root.path(), &FsharpDelegateBaseFixtures::OVERLOAD_FILES)
        .await;
    let report = run_driver(root.path(), FsharpDelegateBaseFixtures::OVERLOAD_DRIVER);
    assert_eq!(
        report["quoted"],
        serde_json::json!({"rate": 4, "total": 30, "extra": 34}),
        "{report}"
    );
    assert_eq!(report["asked"], 1, "{report}");
}
