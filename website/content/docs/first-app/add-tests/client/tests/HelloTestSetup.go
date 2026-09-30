// @TestSetup
type HelloTestSetup struct{}

func (HelloTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}
