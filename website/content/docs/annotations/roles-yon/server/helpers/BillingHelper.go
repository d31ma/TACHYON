// @Helper
type BillingHelper struct{}

func (helper *BillingHelper) Settle(subtotal int) int {
	const surcharge = 2
	return subtotal + surcharge
}
