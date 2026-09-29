// @Helper
// @Lifetime("transient")
type LedgerHelper struct {
	// @Needs
	amounts *LedgerAmountsHelper
}

func (helper *LedgerHelper) Settle(subtotal int) int {
	surcharge := helper.amounts.Surcharge()
	return subtotal + surcharge
}
