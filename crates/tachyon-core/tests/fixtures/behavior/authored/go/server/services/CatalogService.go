import "sync/atomic"

// @CacheSettings({"name":"products","ttlMs":1000,"maxBytes":4096,"maxEntries":10})
// @Service
type CatalogService struct {
	behaviors *BehaviorContext
	calls     *atomic.Int64
}

// @Cached
func (catalog *CatalogService) List(
	// @PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})
	page BehaviorValue,
) (BehaviorValue, error) {
	count := catalog.calls.Add(1)
	return map[string]any{"page": page, "calls": count}, nil
}

// @Cached
func (catalog *CatalogService) Price(
	// @CacheKey
	sku BehaviorValue,
	note BehaviorValue,
) (BehaviorValue, error) {
	return catalog.calls.Add(1), nil
}

// @UpdateCache({"method":"List"})
func (catalog *CatalogService) Refresh(page BehaviorValue, value BehaviorValue) (BehaviorValue, error) {
	return value, nil
}

// @ClearCache({"method":"List"})
func (catalog *CatalogService) Invalidate(page BehaviorValue) (BehaviorValue, error) {
	return true, nil
}

// @Cached
func (catalog *CatalogService) Validated(
	// @Check(ProductInput)
	input BehaviorValue,
) (BehaviorValue, error) {
	return input, nil
}
