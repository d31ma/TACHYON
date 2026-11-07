package main

// @RequestRoute("/orders")
// @Public
type OrdersController struct{}

// @GetRoute("/{id}")
func (controller *OrdersController) GetOrder(
	// @FromPath
	id int,
) int {
	return id
}
