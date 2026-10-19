// @HTTPClient("/api")
type CatalogClient interface {
	// @GetRequest("/products/{id}")
	Find(
		// @PathVariable("id")
		id BehaviorValue,
		// @RequestParam("view")
		view BehaviorValue,
		// @RequestHeader("X-Tenant")
		tenant BehaviorValue,
	) (BehaviorValue, error)

	// @PostRequest("/products")
	Save(
		// @RequestBody
		product BehaviorValue,
	) (BehaviorValue, error)

	// @PutRequest("/products/{id}")
	Replace(
		// @PathVariable("id")
		id BehaviorValue,
		// @RequestBody
		product BehaviorValue,
	) (BehaviorValue, error)

	// @PatchRequest("/products/{id}")
	Rename(
		// @PathVariable("id")
		id BehaviorValue,
		// @RequestBody
		product BehaviorValue,
	) (BehaviorValue, error)

	// @DeleteRequest("/products/{id}")
	Remove(
		// @PathVariable("id")
		id BehaviorValue,
	) (BehaviorValue, error)
}
