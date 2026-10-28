// @Delegate
// @Lifetime("transient")
type LedgerDelegate struct {
	// @Needs
	amounts *LedgerAmountsDelegate
}

func (helper *LedgerDelegate) Settle(subtotal int) int {
	surcharge := helper.amounts.Surcharge()
	return subtotal + surcharge
}
