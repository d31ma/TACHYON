// @HTTPClient
type CatalogClient interface {
	// @GetRequest("/products")
	All(
		// @RequestParam("page")
		page BehaviorValue,
	) (BehaviorValue, error)

	// @PostRequest("/products")
	Add(
		// @RequestBody
		product BehaviorValue,
	) (BehaviorValue, error)
}
