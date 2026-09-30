// @HTTPClient
type CatalogClient interface {
	// @GetRequest("/api/products")
	All() (BehaviorValue, error)
}
