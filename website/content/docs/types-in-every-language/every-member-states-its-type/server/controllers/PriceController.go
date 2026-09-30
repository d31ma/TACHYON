// @RequestRoute("/price")
// @Public
type PriceController struct {
	// @Needs
	priceService *PriceService
}

// @GetRoute
func (controller *PriceController) GetPrice() Price {
	return controller.priceService.PriceFor(20, 3)
}
