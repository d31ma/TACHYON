#[tac::setup]
struct CatalogSetup {}
impl CatalogSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        struct CatalogTransport;
        impl BehaviorHttpAdapter for CatalogTransport {
            fn exchange(
                &self,
                _request: &BehaviorHttpRequest,
                _execution: &BehaviorExecution,
            ) -> BehaviorResult<BehaviorValue> {
                Ok(BehaviorValue::Array(vec!["anvil".into(), "rope".into()]))
            }
        }

        let mut configuration = BehaviorConfiguration::default();
        configuration
            .http
            .insert("catalog".into(), std::sync::Arc::new(CatalogTransport));
        Ok(configuration)
    }
}
