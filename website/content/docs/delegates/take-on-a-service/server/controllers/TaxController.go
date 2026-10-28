// @RequestRoute("/tax")
// @Public
type TaxController struct {
	// @Needs
	taxDelegate *TaxDelegate
}

// @GetRoute
func (controller *TaxController) GetTax() TaxAnswer {
	return TaxAnswer{
		Rate:  controller.taxDelegate.Rate("north"),
		Total: controller.taxDelegate.Total(3),
		Extra: controller.taxDelegate.TotalWith(3, 4),
	}
}
