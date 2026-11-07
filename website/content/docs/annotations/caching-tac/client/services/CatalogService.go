// @Service
// @CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
type CatalogService struct {
	behaviors *BehaviorContext
	Sku       string
	Reads     int
}

// @OnStart
func (catalog *CatalogService) Start() error {
	catalog.Sku = "anvil"
	options := NewBehaviorOptions("public-catalog", NewBehaviorSignal())
	context, err := BehaviorsCreate[CatalogSetup](options)
	catalog.behaviors = context
	return err
}

// @Cached
func (catalog *CatalogService) Find(sku BehaviorValue) (BehaviorValue, error) {
	catalog.Reads++
	return catalog.entry(sku, catalog.Reads), nil
}

// @UpdateCache({"method":"Find"})
func (catalog *CatalogService) Write(sku BehaviorValue, reads BehaviorValue) (BehaviorValue, error) {
	return catalog.entry(sku, reads), nil
}

// @ClearCache({"method":"Find"})
func (catalog *CatalogService) Invalidate(sku BehaviorValue) (BehaviorValue, error) {
	return catalog.entry(sku, catalog.Reads), nil
}

func (catalog *CatalogService) entry(sku BehaviorValue, reads any) map[string]any {
	return map[string]any{"sku": sku, "reads": reads}
}
