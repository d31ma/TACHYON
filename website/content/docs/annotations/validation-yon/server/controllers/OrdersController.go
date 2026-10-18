// @RequestRoute("/orders")
// @Public
type OrdersController struct {
	// @Needs
	orders *OrderService
}

// @PostRoute
func (controller *OrdersController) PostOrders(
	// @FromBody
	order any,
) (any, error) {
	return controller.orders.Accept(order)
}
