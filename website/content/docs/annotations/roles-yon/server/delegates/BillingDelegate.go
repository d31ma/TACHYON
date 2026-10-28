// @Delegate
type BillingDelegate struct{}

func (helper *BillingDelegate) Settle(subtotal int) int {
	const surcharge = 2
	return subtotal + surcharge
}
