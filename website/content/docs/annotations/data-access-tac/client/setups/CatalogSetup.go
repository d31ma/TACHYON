// @Setup
type CatalogSetup struct{}

func (CatalogSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	config := NewBehaviorConfiguration()
	config.Entities["publicCatalog"] = CatalogSetup{}
	config.AuthorizeEntity = func(command *BehaviorEntityCommand, execution *BehaviorExecution) (bool, error) {
		return command.Entity == "ProductEntity", nil
	}
	return config, nil
}

func (CatalogSetup) Execute(command *BehaviorEntityCommand, execution *BehaviorExecution) (BehaviorValue, error) {
	if err := execution.Check(); err != nil {
		return nil, err
	}
	query, isQuery := command.Query.(map[string]any)
	if !isQuery {
		return nil, &BehaviorError{Code: "invalid", Message: "the command has no query"}
	}
	if query["kind"] == "count" {
		return 0, nil
	}
	return []any{}, nil
}
