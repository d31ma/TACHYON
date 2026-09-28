// @Store
// @Lifetime("singleton")
type LedgerStore struct{}

func (repository *LedgerStore) Subtotal() int {
	return 40
}

func (repository *LedgerStore) Surcharge() int {
	return 2
}
