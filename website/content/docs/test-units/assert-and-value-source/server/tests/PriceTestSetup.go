// @TestSetup
type PriceTestSetup struct{}

func (PriceTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}
