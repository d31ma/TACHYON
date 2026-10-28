// @RequestRoute("/checkout")
// @Public
type CheckoutController struct {
	// @Needs
	service *CheckoutService
}

// @GetRoute
func (controller *CheckoutController) GetCheckout() Receipt {
	return Receipt{Total: controller.service.Total()}
}
