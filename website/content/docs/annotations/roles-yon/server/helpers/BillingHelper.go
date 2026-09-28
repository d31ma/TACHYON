// @Helper
type BillingHelper struct {
	// @Needs
	client *BillingClient
}

func (helper *BillingHelper) Settle(subtotal int) int {
	surcharge := helper.client.Surcharge()
	return subtotal + surcharge
}
