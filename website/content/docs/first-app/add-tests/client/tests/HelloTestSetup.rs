#[TestSetup]
struct HelloTestSetup {}
impl HelloTestSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        struct GreetingFake;
        impl BehaviorHttpAdapter for GreetingFake {
            fn exchange(&self, _request: &BehaviorHttpRequest, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
                Ok(BehaviorValue::object([("message", BehaviorValue::from("Hi"))]))
            }
        }

        let mut configuration = BehaviorConfiguration::default();
        configuration.http.insert("greeting".into(), std::sync::Arc::new(GreetingFake));
        Ok(configuration)
    }
}
