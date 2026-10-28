use super::script_named_needs_tests::ScriptNamedNeeds;

struct ScriptDelegateBase;

impl ScriptDelegateBase {
    const PAGE: [(&'static str, &'static str); 2] = [
        ("client/pages/tac.html", "<quote-panel />"),
        (
            "client/components/quote/panel/tac.html",
            "<p>{quote.quote()} {tax.asked}</p>",
        ),
    ];

    const JAVASCRIPT: [(&'static str, &'static str); 3] = [
        (
            "client/services/QuoteService.js",
            "@Service\nexport default class QuoteService {\n  /** @type {TaxDelegate} */\n  @Needs\n  taxDelegate;\n\n  quote() {\n    return this.taxDelegate.rate('north') + this.taxDelegate.total(3);\n  }\n}\n",
        ),
        (
            "client/services/TaxService.js",
            "@Service\nexport default class TaxService {\n  asked = 0;\n\n  rate(region) {\n    this.asked += 1;\n    return 2;\n  }\n\n  total(count) {\n    return count * 10;\n  }\n}\n",
        ),
        (
            "client/delegates/TaxDelegate.js",
            "@Delegate\nexport default class TaxDelegate {\n  /** @type {TaxService} */\n  @Needs\n  tax;\n\n  rate(region) {\n    return this.tax.rate(region) * 2;\n  }\n}\n",
        ),
    ];

    const TYPESCRIPT: [(&'static str, &'static str); 3] = [
        (
            "client/services/QuoteService.ts",
            "@Service\nexport default class QuoteService {\n  @Needs\n  taxDelegate!: TaxDelegate;\n\n  quote(): number {\n    return this.taxDelegate.rate('north') + this.taxDelegate.total(3);\n  }\n}\n",
        ),
        (
            "client/services/TaxService.ts",
            "@Service\nexport default class TaxService {\n  asked = 0;\n\n  rate(region: string): number {\n    this.asked += 1;\n    return 2;\n  }\n\n  total(count: number): number {\n    return count * 10;\n  }\n}\n",
        ),
        (
            "client/delegates/TaxDelegate.ts",
            "@Delegate\nexport default class TaxDelegate {\n  @Needs\n  tax!: TaxService;\n\n  rate(region: string): number {\n    return this.tax.rate(region) * 2;\n  }\n}\n",
        ),
    ];

    const DRIVER: &'static str = "const open = async (name) => {\n\
         \x20 const module = await import(`./.tachyon/services/${name}Service.js`)\n\
         \x20 return [module, await module.__tachyon_create_owner__({}, new AbortController().signal, undefined)]\n\
         }\n\
         const [quoteModule, quote] = await open('Quote')\n\
         const [, tax] = await open('Tax')\n\
         const filled = []\n\
         for (const member of quoteModule.__tachyon_event_owners__(quote)) {\n\
         \x20 for (const key of Object.keys(member)) {\n\
         \x20   if (key !== 'tax' || member[key] !== undefined) continue\n\
         \x20   member[key] = tax\n\
         \x20   filled.push(member.constructor.name)\n\
         \x20 }\n\
         }\n\
         const quoted = quote.quote()\n\
         console.log(JSON.stringify({ quoted, asked: tax.asked, filled }))\n";

    async fn report(base: &[(&'static str, &'static str)]) -> serde_json::Value {
        let files = Self::PAGE.iter().chain(base).copied().collect::<Vec<_>>();
        ScriptNamedNeeds::run(&files, Self::DRIVER).await
    }

    fn assert_shared(report: &serde_json::Value) {
        assert_eq!(report["quoted"], 34, "{report}");
        assert_eq!(report["asked"], 1, "{report}");
        assert_eq!(
            report["filled"],
            serde_json::json!(["TaxDelegate"]),
            "{report}"
        );
    }
}

#[tokio::test]
async fn a_javascript_delegate_waits_for_the_service_that_the_page_shares() {
    if !crate::node_driver::NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = ScriptDelegateBase::report(&ScriptDelegateBase::JAVASCRIPT).await;
    ScriptDelegateBase::assert_shared(&report);
}

#[tokio::test]
async fn a_typescript_delegate_waits_for_the_service_that_the_page_shares() {
    if !crate::node_driver::NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = ScriptDelegateBase::report(&ScriptDelegateBase::TYPESCRIPT).await;
    ScriptDelegateBase::assert_shared(&report);
}
