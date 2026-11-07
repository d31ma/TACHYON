// @Setup
type CatalogSetup struct{}

func (CatalogSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	configuration := NewBehaviorConfiguration()
	configuration.Caches["catalog"] = &MemoryCacheDelegate{entries: map[string]BehaviorCacheEntry{}}
	configuration.Authorize = func(call *BehaviorAuthorizationCall, execution *BehaviorExecution) (bool, error) {
		return true, nil
	}
	return configuration, nil
}
