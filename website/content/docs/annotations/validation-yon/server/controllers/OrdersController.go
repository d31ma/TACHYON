import (
	"io"
	"net/http"
	"strings"
)

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
) any {
	checked, err := controller.orders.Accept(order)
	return controller.answer(checked, err)
}

func (controller *OrdersController) answer(checked any, failure error) any {
	if failure != nil {
		header := http.Header{"Content-Type": {"application/json"}}
		body := io.NopCloser(strings.NewReader(`{"error":"validation"}`))
		return &http.Response{StatusCode: http.StatusBadRequest, Header: header, Body: body}
	}
	return checked
}
