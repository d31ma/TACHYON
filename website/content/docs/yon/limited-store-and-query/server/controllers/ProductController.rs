use crate::models::ProductReport;

#[RequestRoute("/products")]
#[Public]
struct ProductController {}
impl ProductController {
    fn new() -> Self { Self {} }
    #[GetRoute]
    fn get_product(&self) -> Result<ProductReport, YonResourceError> {
        let options = BehaviorOptions::new("public-catalog", BehaviorSignal::new());
        let context = Behaviors::create::<CatalogSetup>(options).map_err(|_| YonResourceError)?;
        let products = ProductStore::new(context.clone());
        let page_request = BehaviorValue::object([("size", BehaviorValue::Number(5.0))]);
        let page = products.bounded(page_request).map_err(|_| YonResourceError)?;
        let active_status = "active";
        let active = products.find_by_status(active_status.into()).map_err(|_| YonResourceError)?;
        let active_count = products.count_by_status(active_status.into()).map_err(|_| YonResourceError)?;
        let others = products.excluding(active_status.into(), page).map_err(|_| YonResourceError)?;
        context.dispose();
        Ok(ProductReport { active: Self::json(active), count: Self::json(active_count), others: Self::json(others) })
    }

    fn json(value: BehaviorValue) -> Json {
        Json::parse(&value.to_json(65536).expect("serialize")).expect("parse")
    }
}
