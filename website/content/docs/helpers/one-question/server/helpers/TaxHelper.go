// @Helper
type TaxHelper struct {
	// @Needs
	store *TaxRateStore
}

func (helper *TaxHelper) AddTax(subtotal int) int {
	tax := helper.store.Rate()
	return subtotal + tax
}
