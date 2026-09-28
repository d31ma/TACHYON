#[yon::setup]
struct CatalogSetup {}
impl CatalogSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        let mut configuration = BehaviorConfiguration::default();
        configuration.caches.insert("catalog".into(), std::sync::Arc::new(MemoryCacheHelper::default()));
        configuration.authorize = Some(std::sync::Arc::new(|_call, _execution| Ok(true)));
        Ok(configuration)
    }
}
