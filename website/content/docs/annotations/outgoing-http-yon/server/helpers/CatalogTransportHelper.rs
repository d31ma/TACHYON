#[Helper]
struct CatalogTransportHelper;
impl BehaviorHttpAdapter for CatalogTransportHelper {
    fn exchange(&self, _request: &BehaviorHttpRequest, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
        Ok(BehaviorValue::object([("items", BehaviorValue::Array(vec!["anvil".into()]))]))
    }
}

impl CatalogTransportHelper {
    fn new() -> Self { Self }
}
