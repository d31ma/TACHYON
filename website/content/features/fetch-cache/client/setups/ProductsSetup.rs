#[tac::setup]
struct ProductsSetup {}
impl ProductsSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        struct ProductsTransport;
        impl BehaviorHttpAdapter for ProductsTransport {
            fn exchange(&self, _request: &BehaviorHttpRequest, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
                Ok(BehaviorValue::object([("items", BehaviorValue::Array(vec!["anvil".into(), "hammer".into()]))]))
            }
        }

        let mut configuration = BehaviorConfiguration::default();
        configuration.http.insert("products".into(), std::sync::Arc::new(ProductsTransport));
        Ok(configuration)
    }
}
