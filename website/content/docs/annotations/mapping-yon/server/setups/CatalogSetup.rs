#[Setup]
struct CatalogSetup {}
impl CatalogSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        Ok(BehaviorConfiguration::default())
    }
}
