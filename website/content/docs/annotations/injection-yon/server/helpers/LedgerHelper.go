// @Helper
// @Lifetime("transient")
type LedgerHelper struct {
	// @Needs
	repository *LedgerStore
}

func (helper *LedgerHelper) Settle(subtotal int) int {
	surcharge := helper.repository.Surcharge()
	return subtotal + surcharge
}
