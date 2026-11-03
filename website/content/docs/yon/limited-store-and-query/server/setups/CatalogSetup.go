// @Setup
type CatalogSetup struct{}

func (CatalogSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	configuration := NewBehaviorConfiguration()
	configuration.Entities["catalog"] = &CatalogAdapterDelegate{}
	configuration.AuthorizeEntity = func(command *BehaviorEntityCommand, execution *BehaviorExecution) (bool, error) {
		return command.Entity == "ProductEntity", nil
	}
	return configuration, nil
}
