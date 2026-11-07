// @TestSetup
type GreetingTestSetup struct{}

func (GreetingTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}
