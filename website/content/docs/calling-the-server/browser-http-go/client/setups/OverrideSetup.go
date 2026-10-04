// @Setup
type OverrideSetup struct{}

func (OverrideSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	config := NewBehaviorConfiguration()
	config.Http["override"] = OverrideSetup{}
	return config, nil
}

func (OverrideSetup) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
	return map[string]any{"message": "Custom adapter wins"}, nil
}
