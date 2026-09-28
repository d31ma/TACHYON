#[yon::setup]
struct CatalogSetup {}
impl CatalogSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        let mut configuration = BehaviorConfiguration::default();
        configuration.http.insert("catalog".into(), std::sync::Arc::new(CatalogTransportHelper));
        Ok(configuration)
    }
}
