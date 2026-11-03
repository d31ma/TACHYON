// @Setup
// @Settings({"prefix":"YON_CATALOG_","defaults":{"pageSize":20,"cacheEnabled":false}})
// @WhenProfile("development")
// @WhenSetting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
type CatalogSetup struct {
	// @Min(1)
	// @Max(100)
	pageSize     int64
	cacheEnabled bool
}

func (CatalogSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}
