use crate::models::CatalogSummary;

#[RequestRoute("/catalog")]
#[Public]
struct CatalogController {}
impl CatalogController {
    fn new() -> Self { Self {} }
    #[GetRoute]
    fn get_catalog(&self) -> Result<CatalogSummary, YonResourceError> {
        let options = BehaviorOptions::new("public-catalog", BehaviorSignal::new());
        let context = Behaviors::create::<CatalogSetup>(options).map_err(|_| YonResourceError)?;
        let mapper = ProductConverter::new(context.clone());
        let product = BehaviorValue::object([
            ("detail", BehaviorValue::object([("id", BehaviorValue::Number(7.0))])),
            ("name", "Anvil".into()),
            ("extra", "dropped".into()),
        ]);
        let summary = mapper.to_summary(product).map_err(|_| YonResourceError)?;
        context.dispose();
        Ok(CatalogSummary { summary: Self::json(summary) })
    }

    fn json(value: BehaviorValue) -> Json {
        Json::parse(&value.to_json(65536).expect("serialize")).expect("parse")
    }
}
