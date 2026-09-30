#[TestSetup]
struct HelloTestSetup {}
impl HelloTestSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        Ok(BehaviorConfiguration::default())
    }
}
