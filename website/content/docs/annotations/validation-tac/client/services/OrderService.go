// @Service
type OrderService struct {
	Status string
}

// @OnStart
func (service *OrderService) Start() {
	service.Status = "ready"
}

func (service *OrderService) Accept(
	// @Check(OrderInput)
	order BehaviorValue,
) (BehaviorValue, error) {
	return order, nil
}
