#[TestSetup]
struct CatalogTestSetup {}
impl CatalogTestSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        Ok(BehaviorConfiguration::default())
    }
}
