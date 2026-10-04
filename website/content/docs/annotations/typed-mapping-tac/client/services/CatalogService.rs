use crate::models::{Detail, Product, Summary};
#[Service]
#[derive(Default)]
struct CatalogService {
    #[Needs]
    product_converter: std::sync::Arc<ProductConverter>,
    result: String,
}
impl CatalogService {
    fn load(&mut self) {
        let detail = Detail {
            name: "detail".into(),
            secret: "detail-private".into(),
        };
        let product = Product {
            id: 7,
            name: "Anvil".into(),
            detail: detail.clone(),
            items: vec![detail],
            secret: "product-private".into(),
        };
        let copied = self.product_converter.copy(product);
        let summary = copied.and_then(|product| self.product_converter.map(product));
        self.result = summary.map(|summary| Self::text(&summary)).unwrap_or_default();
    }

    fn text(summary: &Summary) -> String {
        format!("{} {}: {}, {}, {}", summary.kind, summary.id, summary.name, summary.detail.name, summary.items.len())
    }
}
