#[TestSetup]
struct OrdersTestSetup {}
impl OrdersTestSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        struct OrdersEcho;
        impl BehaviorHttpAdapter for OrdersEcho {
            fn exchange(&self, request: &BehaviorHttpRequest, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
                let header = match &request.headers {
                    BehaviorValue::Object(headers) => headers.values().next().cloned().unwrap_or(BehaviorValue::Null),
                    _ => BehaviorValue::Null,
                };
                let item = request.body.as_ref().and_then(|body| body.get("item")).cloned().unwrap_or_else(|| BehaviorValue::from("none"));
                Ok(BehaviorValue::object([
                    ("method", BehaviorValue::from(request.method)),
                    ("path", BehaviorValue::from(request.path.as_str())),
                    ("header", header),
                    ("item", item),
                ]))
            }
        }

        let mut configuration = BehaviorConfiguration::default();
        configuration.http.insert("orders".into(), std::sync::Arc::new(OrdersEcho));
        Ok(configuration)
    }
}
