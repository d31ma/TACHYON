use crate::models::{Detail, Product, Summary};
#[RequestRoute("/catalog")]
#[Public]
struct CatalogController {
    #[Needs]
    converter: std::sync::Arc<ProductConverter>,
}
impl CatalogController {
    #[GetRoute]
    fn get_catalog(&self) -> Result<Summary, YonResourceError> {
        let detail = Detail {
            name: "detail".into(),
            secret: "hidden".into(),
        };
        self.converter
            .map(Product {
                id: 7,
                name: "Anvil".into(),
                detail: detail.clone(),
                items: vec![detail],
                secret: "hidden".into(),
            })
            .map_err(|_| YonResourceError)
    }
}
