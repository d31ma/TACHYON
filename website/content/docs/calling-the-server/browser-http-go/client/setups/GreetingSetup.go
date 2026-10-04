// @Setup
type GreetingSetup struct{}

func (GreetingSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}
