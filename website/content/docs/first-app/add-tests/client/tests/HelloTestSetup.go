// @TestSetup
type HelloTestSetup struct{}

func (HelloTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	config := NewBehaviorConfiguration()
	config.Http["greeting"] = HelloTestSetup{}
	return config, nil
}

func (HelloTestSetup) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
	return map[string]any{"message": "Hi"}, nil
}
