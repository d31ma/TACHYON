use super::PythonWebServices;
use super::backend_tests::PythonPage as PythonBackend;
use super::page_tests::PythonPage;
use std::path::PathBuf;

struct PythonConverters;

impl PythonConverters {
    const FILES: [(&'static str, &'static str); 7] = [
        (
            "client/models/Detail.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass Detail:\n    name: str\n    secret: str\n",
        ),
        (
            "client/models/ShortDetail.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass ShortDetail:\n    name: str\n",
        ),
        (
            "client/models/Product.py",
            "from dataclasses import dataclass\n\nfrom models import Detail\n\n\n@dataclass\nclass Product:\n    id: int\n    name: str\n    detail: Detail\n    items: list[Detail]\n    secret: str\n",
        ),
        (
            "client/models/Summary.py",
            "from dataclasses import dataclass\n\nfrom models import ShortDetail\n\n\n@dataclass\nclass Summary:\n    id: int\n    name: str\n    detail: ShortDetail\n    items: list[ShortDetail]\n    kind: str\n",
        ),
        (
            "client/converters/ProductConverter.py",
            "from typing import Protocol\n\nfrom models import Product, Summary\n\n\n@converter\nclass ProductConverter(Protocol):\n    @Convert({\"fields\":[{\"target\":\"kind\",\"constant\":\"product\"}]})\n    def map(self, product: Product) -> Summary: ...\n\n    def copy(self, product: Product) -> Product: ...\n",
        ),
        (
            "client/converters/CatalogConverter.py",
            "from typing import Protocol\n\n\n@converter\nclass CatalogConverter(Protocol):\n    @Convert({\"fields\":[{\"source\":\"detail.id\",\"target\":\"productId\"},{\"source\":\"name\",\"target\":\"title\"},{\"source\":\"missing.deep\",\"target\":\"absent\"},{\"target\":\"kind\",\"constant\":\"product\"}]})\n    def to_summary(self, product: dict[str, object]) -> dict[str, object]: ...\n\n    @Convert({\"fields\":[{\"source\":\"name\",\"target\":\"title\"}]})\n    async def to_title(self, product: dict[str, object]) -> dict[str, object]: ...\n",
        ),
        (
            "client/services/CatalogService.py",
            "from typing import Annotated\n\nfrom models import Detail, Product\n\n\n@service\nclass CatalogService:\n    product_converter: Annotated[ProductConverter, needs]\n    catalog_converter: Annotated[CatalogConverter, needs]\n\n    def typed(self) -> str:\n        detail = Detail('detail', 'detail-private')\n        product = self.product_converter.copy(Product(7, 'Anvil', detail, [detail], 'product-private'))\n        summary = self.product_converter.map(product)\n        return type(product).__name__ + ' ' + type(summary).__name__ + ':' + str(summary)\n\n    def plain(self) -> dict[str, object]:\n        return self.catalog_converter.to_summary({'detail': {'id': 7}, 'name': 'Anvil', 'extra': 'dropped'})\n\n    async def title(self) -> dict[str, object]:\n        return await self.catalog_converter.to_title({'name': 'Anvil', 'extra': 'dropped'})\n",
        ),
    ];

    const TYPED: &'static str = "Product Summary:Summary(id=7, name='Anvil', detail=ShortDetail(name='detail'), items=[ShortDetail(name='detail')], kind='product')";

    const PAGE_DRIVER: &'static str = "console.info = () => {}\n\
         const { default: CatalogService } = await import('./CatalogService.js')\n\
         const catalog = new CatalogService({})\n\
         const typed = await catalog.typed()\n\
         const plain = await catalog.plain()\n\
         const title = await catalog.title()\n\
         console.log(JSON.stringify({ typed, plain, title }))\n\
         process.exit(0)\n";

    const BACKEND_DRIVER: &'static str = "_report = {'init': _ask('catalog', {'op': 'init', 'props': {}}),\n\
         \x20   'typed': _ask('catalog', {'op': 'call', 'name': 'typed', 'args': []}),\n\
         \x20   'plain': _ask('catalog', {'op': 'call', 'name': 'plain', 'args': []})}\n";

    const WITH_BODY: &'static str = "from typing import Protocol\n\n\n@converter\nclass CatalogConverter(Protocol):\n    @Convert({\"fields\":[{\"source\":\"name\",\"target\":\"title\"}]})\n    def to_title(self, product: dict[str, object]) -> dict[str, object]:\n        return {'title': product['name']}\n";

    fn plain_summary() -> serde_json::Value {
        serde_json::json!({"productId": 7, "title": "Anvil", "absent": null, "kind": "product"})
    }
}

#[test]
fn a_python_service_runs_its_converter_mappings_under_cpython() {
    if !PythonBackend::has_python() {
        return;
    }
    let report = PythonBackend::run(
        &PythonConverters::FILES,
        &["catalog"],
        PythonConverters::BACKEND_DRIVER,
    );
    assert_eq!(
        report["typed"]["value"],
        PythonConverters::TYPED,
        "{report}"
    );
    assert_eq!(
        report["plain"]["value"],
        PythonConverters::plain_summary(),
        "{report}"
    );
}

#[tokio::test]
async fn a_python_service_needs_a_converter_and_runs_its_mappings_in_pyodide() {
    if let Some(reason) = PythonPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    let (service, source) = PythonConverters::FILES[6];
    for (relative, text) in &PythonConverters::FILES[..6] {
        PythonPage::write(root.path(), relative, text);
    }
    assert_eq!(service, "client/services/CatalogService.py");
    PythonPage::publish(root.path(), &[("catalog", "CatalogService", source)]).await;
    let report = PythonPage::run(root.path(), PythonConverters::PAGE_DRIVER);
    assert_eq!(report["typed"], PythonConverters::TYPED, "{report}");
    assert_eq!(
        report["plain"],
        PythonConverters::plain_summary(),
        "{report}"
    );
    assert_eq!(
        report["title"],
        serde_json::json!({"title": "Anvil"}),
        "{report}"
    );
}

#[test]
fn a_python_converter_with_a_method_body_is_refused() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    let (service, _) = PythonConverters::FILES[6];
    for (relative, text) in PythonConverters::FILES {
        PythonPage::write(root.path(), relative, text);
    }
    PythonPage::write(
        root.path(),
        "client/converters/CatalogConverter.py",
        PythonConverters::WITH_BODY,
    );
    let chosen = [(
        String::from("catalog"),
        String::from("CatalogService"),
        root.path().join(PathBuf::from(service)),
    )];
    let failure = PythonWebServices::program(root.path(), &chosen)
        .err()
        .unwrap_or_else(|| panic!("a converter body must be refused"));
    assert_eq!(failure.diagnostics()[0].code.number(), 1308, "{failure}");
    assert!(
        failure
            .to_string()
            .contains("declare its signature with an empty body"),
        "{failure}"
    );
}
