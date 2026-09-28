import "sync/atomic"

// @CacheSettings({"name":"products","ttlMs":1000,"maxBytes":4096,"maxEntries":10})
// @HTTPClient
type CatalogClient struct {
	behaviors *BehaviorContext
	calls     *atomic.Int64
}

// @Cached
func (catalog *CatalogClient) List(
	// @PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})
	page BehaviorValue,
) (BehaviorValue, error) {
	count := catalog.calls.Add(1)
	return map[string]any{"page": page, "calls": count}, nil
}

// @GetRequest("/products")
func (catalog *CatalogClient) Remote() (BehaviorValue, error) {
	return map[string]any{"query": map[string]any{"sort": "id"}}, nil
}

// @GetRequest("/products")
func (catalog *CatalogClient) Search(
	// @RequestParam("page")
	page BehaviorValue,
) (BehaviorValue, error) {
}

// @GetRequest("/products/{id}")
func (catalog *CatalogClient) Find(
	// @PathVariable
	id BehaviorValue,
) (BehaviorValue, error) {
}

// @UpdateCache({"method":"List"})
func (catalog *CatalogClient) Refresh(page BehaviorValue, value BehaviorValue) (BehaviorValue, error) {
	return value, nil
}

// @ClearCache({"method":"List"})
func (catalog *CatalogClient) Invalidate(page BehaviorValue) (BehaviorValue, error) {
	return true, nil
}

// @UpdateCache({"method":"List"})
// @GetRequest("/products")
func (catalog *CatalogClient) RefreshRemote(page BehaviorValue) (BehaviorValue, error) {
	return map[string]any{}, nil
}

// @Cached
func (catalog *CatalogClient) Validated(
	// @Check(ProductInput)
	input BehaviorValue,
) (BehaviorValue, error) {
	return input, nil
}
