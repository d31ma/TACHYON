type CheckoutService struct {
	Placed                    int64
	applicationEventPublisher ApplicationEventPublisher
}

// @OnStart
func (checkout *CheckoutService) Start() {
	checkout.Place()
}

func (checkout *CheckoutService) Place() {
	checkout.Placed++
	order := PlacedOrder{Status: "paid", Total: 25}
	checkout.applicationEventPublisher.PublishEvent(order, true)
}

// @OnStop
func (checkout *CheckoutService) Stop() {
	checkout.Placed = 0
}
