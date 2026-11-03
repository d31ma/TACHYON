#[Delegate]
struct CatalogTransportDelegate;
impl BehaviorHttpAdapter for CatalogTransportDelegate {
    fn exchange(&self, _request: &BehaviorHttpRequest, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
        Ok(BehaviorValue::object([("items", BehaviorValue::Array(vec!["anvil".into()]))]))
    }
}

impl CatalogTransportDelegate {
    fn new() -> Self { Self }
}
