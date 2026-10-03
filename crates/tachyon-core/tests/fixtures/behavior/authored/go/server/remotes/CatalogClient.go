// @HTTPClient({"setup": "ConfigSetup"})
type CatalogClient interface {
	// @GetRequest("/products")
	Remote() (BehaviorValue, error)

	// @GetRequest("/products")
	Search(
		// @RequestParam("page")
		page BehaviorValue,
	) (BehaviorValue, error)

	// @GetRequest("/products/{id}")
	Find(
		// @PathVariable
		id BehaviorValue,
	) (BehaviorValue, error)
}
