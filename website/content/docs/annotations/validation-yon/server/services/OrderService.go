// @Service
type OrderService struct{}

func (service *OrderService) Accept(
	// @Check(OrderInput)
	order BehaviorValue,
) (BehaviorValue, error) {
	return order, nil
}
