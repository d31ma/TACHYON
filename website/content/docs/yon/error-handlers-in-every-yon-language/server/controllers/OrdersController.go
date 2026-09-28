import "fmt"

// @RequestRoute("/orders")
// @Public
type OrdersController struct{}

// @GetRoute
func (controller *OrdersController) GetOrder() (Problem, error) {
	return Problem{}, fmt.Errorf("order 42: %w", &OrderNotFound{})
}
