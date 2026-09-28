// @Setup
type CatalogSetup struct{}

func (CatalogSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	config := NewBehaviorConfiguration()
	config.Http["catalog"] = CatalogSetup{}
	return config, nil
}

func (CatalogSetup) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
	return map[string]any{"items": []any{"anvil"}}, nil
}
