// @Helper
type TaxHelper struct {
	// @Needs
	rates *TaxRateHelper
}

func (helper *TaxHelper) AddTax(subtotal int) int {
	tax := helper.rates.Rate()
	return subtotal + tax
}
