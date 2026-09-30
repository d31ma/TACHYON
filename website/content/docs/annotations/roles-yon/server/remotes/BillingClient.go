// @HTTPClient("/billing")
type BillingClient interface {
	// @GetRequest("/surcharge")
	Surcharge() (BehaviorValue, error)
}
