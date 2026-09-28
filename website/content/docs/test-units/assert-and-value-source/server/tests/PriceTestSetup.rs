#[TestSetup]
struct PriceTestSetup {}
impl PriceTestSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        Ok(BehaviorConfiguration::default())
    }
}
