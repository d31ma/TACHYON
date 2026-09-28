#[Setup]
#[WhenProfile("development", "test")]
#[WhenSetting({"name":"YON_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})]
struct SelectedConfigSetup {}

impl SelectedConfigSetup {
    fn configure(execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        assert_eq!(execution.partition, "configured", "Inactive configuration executed");
        Ok(BehaviorConfiguration::default())
    }
}

