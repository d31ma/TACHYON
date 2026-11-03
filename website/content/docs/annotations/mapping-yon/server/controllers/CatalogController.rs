use crate::models::CatalogSummary;

#[RequestRoute("/catalog")]
#[Public]
struct CatalogController {
    #[Needs]
    converter: std::sync::Arc<ProductConverter>,
}
impl CatalogController {
    #[GetRoute]
    fn get_catalog(&self) -> Result<CatalogSummary, YonResourceError> {
        let product = BehaviorValue::object([
            ("detail", BehaviorValue::object([("id", BehaviorValue::Number(7.0))])),
            ("name", "Anvil".into()),
            ("extra", "dropped".into()),
        ]);
        let summary = self.converter.to_summary(product).map_err(|_| YonResourceError)?;
        Ok(CatalogSummary { summary: Self::json(summary) })
    }

    fn json(value: BehaviorValue) -> Json {
        Json::parse(&value.to_json(65536).expect("serialize")).expect("parse")
    }
}
