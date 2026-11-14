import "errors"

// @RequestRoute("/orders")
// @Allow(Admin)
type OrdersController struct {
	// @Needs
	orders *OrderService
}

// @GetRoute
func (controller *OrdersController) GetOrders() ([]string, error) {
	return nil, errors.New("the order store is down")
}

// @PostRoute
func (controller *OrdersController) PostOrder(
	// @FromBody
	order any,
) (any, error) {
	return controller.orders.Accept(order)
}
