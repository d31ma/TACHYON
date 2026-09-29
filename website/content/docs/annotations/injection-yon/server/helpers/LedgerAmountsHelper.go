// @Helper
// @Lifetime("singleton")
type LedgerAmountsHelper struct{}

func (helper *LedgerAmountsHelper) Subtotal() int {
	return 40
}

func (helper *LedgerAmountsHelper) Surcharge() int {
	return 2
}
