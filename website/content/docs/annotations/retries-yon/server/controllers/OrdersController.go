import "errors"

// @RequestRoute("/orders")
// @Public
type OrdersController struct {
	// @Needs
	orders *OrdersService
}

// @GetRoute
func (controller *OrdersController) GetOrders() (OrderReport, error) {
	id := "A-1"
	submitted, submitErr := controller.orders.Submit(id)
	report, reportErr := controller.orders.Report(id)
	return OrderReport{Submitted: submitted, Report: report}, errors.Join(submitErr, reportErr)
}
