// @Delegate
type TaxDelegate struct {
	// @Needs
	rates *TaxRateDelegate
}

func (helper *TaxDelegate) AddTax(subtotal int) int {
	tax := helper.rates.Rate()
	return subtotal + tax
}
