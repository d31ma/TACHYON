// @HTTPClient
type BillingClient struct{}

func (client *BillingClient) Surcharge() int {
	return 2
}
