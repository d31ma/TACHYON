// @CacheSettings({"name":"products","ttlMs":60000,"maxBytes":65536,"maxEntries":16})
// @HTTPClient
type ProductsClient interface {
	// @Cached
	// @GetRequest("/api/products")
	List() (BehaviorValue, error)

	// @ClearCache({"method":"List"})
	// @PostRequest("/api/products")
	Save(
		// @RequestBody
		product BehaviorValue,
	) (BehaviorValue, error)
}
