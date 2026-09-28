// @Service
// @CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
type CatalogService struct {
	behaviors *BehaviorContext
	reads     float64
}

// @OnStart
func (service *CatalogService) Open() error {
	options := NewBehaviorOptions("public-catalog", NewBehaviorSignal())
	context, err := BehaviorsCreate[CatalogSetup](options)
	service.behaviors = context
	return err
}

// @Cached
func (service *CatalogService) Find(sku BehaviorValue) (BehaviorValue, error) {
	service.reads++
	return service.entry(sku, service.reads), nil
}

// @UpdateCache({"method":"Find"})
func (service *CatalogService) Write(sku BehaviorValue, reads BehaviorValue) (BehaviorValue, error) {
	return service.entry(sku, reads), nil
}

// @ClearCache({"method":"Find"})
func (service *CatalogService) Invalidate(sku BehaviorValue) (BehaviorValue, error) {
	return service.entry(sku, service.reads), nil
}

func (service *CatalogService) entry(sku BehaviorValue, reads any) map[string]any {
	return map[string]any{"sku": sku, "reads": reads}
}
