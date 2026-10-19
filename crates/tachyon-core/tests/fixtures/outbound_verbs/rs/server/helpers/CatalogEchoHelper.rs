#[Helper]
struct CatalogEchoHelper;
impl BehaviorHttpAdapter for CatalogEchoHelper {
    fn exchange(&self, request: &BehaviorHttpRequest, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
        Ok(BehaviorValue::object([
            ("method", BehaviorValue::from(request.method)),
            ("path", BehaviorValue::from(request.path.clone())),
            ("query", request.query.clone()),
            ("headers", request.headers.clone()),
            ("body", request.body.clone().unwrap_or(BehaviorValue::Null)),
        ]))
    }
}

impl CatalogEchoHelper {
    fn new() -> Self { Self }
}
