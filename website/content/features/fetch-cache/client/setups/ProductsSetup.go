// @Setup
type ProductsSetup struct{}

func (ProductsSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	config := NewBehaviorConfiguration()
	config.Http["products"] = ProductsSetup{}
	return config, nil
}

func (ProductsSetup) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
	return map[string]any{"items": []any{"anvil", "hammer"}}, nil
}
