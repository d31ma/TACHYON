// @RequestRoute("/orders")
// @SignedIn
type OrdersController struct {
}

// @GetRoute
// @Public
func (controller *OrdersController) GetOrders() []string {
	return []string{"lamp", "desk"}
}

// @PostRoute
func (controller *OrdersController) PostOrder() []string {
	return []string{"chair"}
}

// @DeleteRoute
// @Allow(Admin)
func (controller *OrdersController) DeleteOrders() []string {
	return []string{}
}
