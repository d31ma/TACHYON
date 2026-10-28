// @Delegate
// @Lifetime("singleton")
type LedgerAmountsDelegate struct{}

func (helper *LedgerAmountsDelegate) Subtotal() int {
	return 40
}

func (helper *LedgerAmountsDelegate) Surcharge() int {
	return 2
}
