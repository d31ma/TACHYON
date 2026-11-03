// @Service
type CheckoutService struct {
	// @Needs
	helper *TaxDelegate
}

func (service *CheckoutService) Total() int {
	subtotal := service.subtotal()
	return service.helper.AddTax(subtotal)
}

func (service *CheckoutService) subtotal() int {
	return 40
}
