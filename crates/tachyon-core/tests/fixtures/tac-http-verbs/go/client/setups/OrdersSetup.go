// @Setup
type OrdersSetup struct{}

func (OrdersSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}
