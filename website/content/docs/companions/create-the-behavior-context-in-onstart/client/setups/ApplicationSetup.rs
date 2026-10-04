#[Setup]
struct ApplicationSetup {}

impl ApplicationSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        Ok(BehaviorConfiguration::default())
    }
}
