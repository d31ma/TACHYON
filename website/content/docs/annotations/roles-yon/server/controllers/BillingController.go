// @RequestRoute("/billing")
// @Public
type BillingController struct {
	// @Needs
	service *BillingService
}

// @GetRoute
func (controller *BillingController) GetBilling() Bill {
	return Bill{Total: controller.service.Total()}
}
