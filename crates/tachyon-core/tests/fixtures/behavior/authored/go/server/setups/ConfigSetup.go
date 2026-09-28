// @Setup
type ConfigSetup struct{}

func (ConfigSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	products := "products"
	tenant := "tenant-a"
	adapter := "adapter"
	memory := &Memory{entries: map[string]BehaviorCacheEntry{}}
	config := NewBehaviorConfiguration()
	config.Caches[products] = memory
	config.Http["catalog"] = memory
	config.Entities["data"] = memory
	config.Authorize = func(call *BehaviorAuthorizationCall, execution *BehaviorExecution) (bool, error) {
		if err := execution.Check(); err != nil {
			return false, err
		}
		if call.Cache != products || call.Owner != "CatalogClient" {
			return false, &BehaviorError{Code: adapter, Message: "unexpected cache authorization"}
		}
		return execution.Partition == tenant, nil
	}
	config.AuthorizeEntity = func(command *BehaviorEntityCommand, execution *BehaviorExecution) (bool, error) {
		if err := execution.Check(); err != nil {
			return false, err
		}
		if command.Operation != "query" || command.Table != products {
			return false, &BehaviorError{Code: adapter, Message: "unexpected entity authorization"}
		}
		return execution.Partition == tenant, nil
	}
	return config, nil
}
