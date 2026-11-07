// @ErrorHandler
type OrdersErrorHandler struct{}

// @HandlesError
// @ResponseStatus(404)
func (handler *OrdersErrorHandler) Missing(failure *OrderNotFound) Problem {
	return Problem{Code: "order-not-found"}
}
