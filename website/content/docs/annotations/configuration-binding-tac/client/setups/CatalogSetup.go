// @Setup
// @Settings({"prefix":"TAC_CATALOG_","defaults":{"pageSize":20,"cacheEnabled":false,"title":"Catalog"}})
// @WhenProfile("public")
// @WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
type CatalogSetup struct {
	// @Min(1)
	// @Max(100)
	pageSize     int64
	cacheEnabled bool
	title        string
}

func (CatalogSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}
