#![allow(clippy::expect_used)]

use super::page_tests::PythonPage;

struct PythonChecks;

impl PythonChecks {
    const FILES: [(&'static str, &'static str); 2] = [
        (
            "client/models/OrderInput.py",
            "from typing import Annotated\n\n@checked\nclass OrderInput:\n    name: Annotated[str, not_empty, length({\"min\":1,\"max\":8})] = ''\n    level: Annotated[int, required, min(1), max(9)] = 0\n",
        ),
        (
            "client/setups/CatalogSetup.py",
            "from typing import Annotated\n\n\n@setup\n@settings({\"prefix\":\"TAC_CATALOG_\"})\n@when_profile(\"public\")\n@when_setting({\"name\":\"TAC_CATALOG_CACHE_ENABLED\",\"havingValue\":\"true\",\"matchIfMissing\":false})\nclass CatalogSetup:\n    pageSize: Annotated[int, min(1), max(100)] = 20\n    cacheEnabled: bool = False\n    title: str = 'Catalog'\n\n    @staticmethod\n    def configure(execution: object) -> dict[str, object]:\n        return {}\n",
        ),
    ];

    const SERVICE: &'static str = "from typing import Annotated\nfrom models import OrderInput\n\n\n@service\nclass OrderService:\n    def accept(self, order: Annotated[OrderInput, check]) -> str:\n        return order['name'] + ':' + str(order['level'])\n\n    def page_size(self, value: str) -> int:\n        return Behaviors.properties(CatalogSetup, {'TAC_CATALOG_PAGE_SIZE': value})['pageSize']\n\n    def refusal(self, value: object) -> str:\n        try:\n            self.accept(value)\n            return 'accepted'\n        except BehaviorRejection as refused:\n            return ','.join(violation['rule'] for violation in refused.violations)\n\n    def default_page_size(self) -> int:\n        return Behaviors.properties(CatalogSetup, {})['pageSize']\n";

    const DRIVER: &'static str = "console.info = () => {}\n\
         const outcome = (run) => { try { return { value: run() } } catch (error) { return { code: error.code ?? '', message: String(error.message) } } }\n\
         const { default: OrderService } = await import('./OrderService.js')\n\
         const orders = new OrderService({})\n\
         const accepted = outcome(() => orders.accept({ name: 'Tea', level: 2 }))\n\
         const values = {\n\
         \x20 notAnObject: 'Tea',\n\
         \x20 emptyName: { name: '', level: 2 },\n\
         \x20 blankName: { name: '   ', level: 2 },\n\
         \x20 longName: { name: 'Teapotted', level: 2 },\n\
         \x20 missingLevel: { name: 'Tea' },\n\
         \x20 lowLevel: { name: 'Tea', level: 0 },\n\
         \x20 highLevel: { name: 'Tea', level: 10 },\n\
         }\n\
         const refused = Object.fromEntries(Object.entries(values).map(([name, value]) => [name, outcome(() => orders.accept(value))]))\n\
         const rules = Object.fromEntries(Object.entries(values).map(([name, value]) => [name, orders.refusal(value)]))\n\
         rules.edges = orders.refusal({ name: 'Teapotte', level: 9 }) + ' ' + orders.refusal({ name: 'T', level: 1 })\n\
         const pageSize = { fallback: outcome(() => orders.defaultPageSize()), low: outcome(() => orders.pageSize('0')), lowest: outcome(() => orders.pageSize('1')), highest: outcome(() => orders.pageSize('100')), high: outcome(() => orders.pageSize('101')) }\n\
         console.log(JSON.stringify({ accepted, refused, rules, pageSize }))\n\
         process.exit(0)\n";
}

#[tokio::test]
async fn a_python_service_checks_its_argument_and_a_setup_bounds_its_settings_on_the_page() {
    if let Some(reason) = PythonPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in PythonChecks::FILES {
        PythonPage::write(root.path(), relative, text);
    }
    PythonPage::publish(
        root.path(),
        &[("orders", "OrderService", PythonChecks::SERVICE)],
    )
    .await;
    let report = PythonPage::run(root.path(), PythonChecks::DRIVER);
    assert_eq!(report["accepted"]["value"], "Tea:2", "{report}");
    let refused = report["refused"].as_object().expect("refused values");
    assert_eq!(refused.len(), 7, "{report}");
    for (case, outcome) in refused {
        assert!(
            outcome.get("value").is_none(),
            "{case} was accepted: {report}"
        );
        assert!(
            outcome["message"]
                .as_str()
                .is_some_and(|message| message.starts_with("Web service orders:")),
            "{case}: {report}"
        );
    }
    assert_eq!(
        report["rules"],
        serde_json::json!({
            "notAnObject": "Checked",
            "emptyName": "NotEmpty,Length",
            "blankName": "NotEmpty",
            "longName": "Length",
            "missingLevel": "Required",
            "lowLevel": "Min",
            "highLevel": "Max",
            "edges": "accepted accepted",
        }),
        "{report}"
    );
    let page_size = &report["pageSize"];
    assert_eq!(page_size["fallback"]["value"], 20, "{report}");
    assert_eq!(page_size["lowest"]["value"], 1, "{report}");
    assert_eq!(page_size["highest"]["value"], 100, "{report}");
    assert!(page_size["low"].get("value").is_none(), "{report}");
    assert!(page_size["high"].get("value").is_none(), "{report}");
}
