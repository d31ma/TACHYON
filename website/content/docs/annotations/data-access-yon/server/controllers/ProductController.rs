use crate::models::ProductReport;

#[RequestRoute("/products")]
#[Public]
struct ProductController {
    #[Needs]
    products: std::sync::Arc<ProductStore>,
}
impl ProductController {
    #[GetRoute]
    fn get_product(&self) -> Result<ProductReport, YonResourceError> {
        let page = BehaviorValue::object([("size", BehaviorValue::Number(5.0))]);
        let active_status = "active";
        let active = self.products.find_by_status(active_status.into()).map_err(|_| YonResourceError)?;
        let active_count = self.products.count_by_status(active_status.into()).map_err(|_| YonResourceError)?;
        let others = self.products.excluding(active_status.into(), page).map_err(|_| YonResourceError)?;
        Ok(ProductReport { active: Self::json(active), count: Self::json(active_count), others: Self::json(others) })
    }

    fn json(value: BehaviorValue) -> Json {
        Json::parse(&value.to_json(65536).expect("serialize")).expect("parse")
    }
}
