#[Setup]
struct OverrideSetup {}
impl OverrideSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        struct CustomAnswer;
        impl BehaviorHttpAdapter for CustomAnswer {
            fn exchange(&self, _request: &BehaviorHttpRequest, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
                Ok(BehaviorValue::object([("message", BehaviorValue::from("Custom adapter wins"))]))
            }
        }

        let mut configuration = BehaviorConfiguration::default();
        configuration.http.insert("override".into(), std::sync::Arc::new(CustomAnswer));
        Ok(configuration)
    }
}
