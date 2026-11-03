// @TestSetup
type StatusTestSetup struct{}

func (StatusTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}
