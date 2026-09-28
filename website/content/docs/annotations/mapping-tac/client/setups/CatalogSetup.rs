#[tac::setup]
struct CatalogSetup {}
impl CatalogSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        let mut configuration = BehaviorConfiguration::default();
        configuration.authorize = Some(std::sync::Arc::new(|_call, _execution| Ok(true)));
        Ok(configuration)
    }
}
