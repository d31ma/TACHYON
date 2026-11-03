// @RequestRoute("/orders")
// @Public
type OrdersController struct {
	// @Needs
	service *OrderService
}

// @GetRoute
func (controller *OrdersController) GetOrders() OrderList {
	return OrderList{Orders: controller.service.List()}
}
