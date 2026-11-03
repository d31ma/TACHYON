type OrderNotFound struct{}

func (failure *OrderNotFound) Error() string {
	return "order not found"
}
