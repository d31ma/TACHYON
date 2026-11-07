// @Setup
type ApplicationSetup struct{}

func (ApplicationSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}
