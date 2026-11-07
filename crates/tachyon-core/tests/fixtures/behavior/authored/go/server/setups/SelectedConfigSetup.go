// @Setup
// @WhenProfile("development", "test")
// @WhenSetting({"name":"YON_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
type SelectedConfigSetup struct{}

func (SelectedConfigSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	if execution.Partition != "configured" {
		return nil, &BehaviorError{Code: "adapter", Message: "Inactive configuration executed"}
	}
	return NewBehaviorConfiguration(), nil
}
