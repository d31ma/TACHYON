// @HTTPClient({"path":"/api","adapter":"orders","setup":"OrdersSetup"})
type OrdersClient interface {
	// @PutRequest("/orders/{id}")
	Replace(
		// @PathVariable("id")
		id string,
		// @RequestBody
		order BehaviorValue,
		// @RequestHeader("X-Tenant")
		tenant string,
	) (SentRequest, error)
	// @PatchRequest("/orders/{id}")
	Change(
		// @PathVariable("id")
		id string,
		// @RequestBody
		change BehaviorValue,
		// @RequestHeader("X-Tenant")
		tenant string,
	) (SentRequest, error)
	// @DeleteRequest("/orders/{id}")
	Remove(
		// @PathVariable("id")
		id string,
		// @RequestHeader("X-Reason")
		reason string,
	) (SentRequest, error)
}
