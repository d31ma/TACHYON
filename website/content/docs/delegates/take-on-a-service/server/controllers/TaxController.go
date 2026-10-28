// @RequestRoute("/tax")
// @Public
type TaxController struct {
	// @Needs
	taxDelegate *TaxDelegate
}

func (TaxController) count() int { return 3 }

// @GetRoute
func (controller *TaxController) GetTax() TaxAnswer {
	return TaxAnswer{
		Rate:  controller.taxDelegate.Rate("north"),
		Total: controller.taxDelegate.Total(controller.count()),
		Extra: controller.taxDelegate.TotalWith(controller.count(), 4),
	}
}
