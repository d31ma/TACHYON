// @HTTPClient("/api")
type OrderClient interface {
	// @PostRequest("/orders")
	Send(
		// @RequestBody
		order OrderInput,
	) (OrderInput, error)
}
