// @Setup
type CatalogSetup struct{}

func (CatalogSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	configuration := NewBehaviorConfiguration()
	configuration.Http["catalog"] = &CatalogTransportHelper{}
	return configuration, nil
}
