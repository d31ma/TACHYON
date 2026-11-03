// @Service
type CheckoutService struct {
	// @Needs
	applicationEventPublisher *ApplicationEventPublisher
}

// @OnStart
func (service *CheckoutService) Start() {
	order := PlacedOrder{Status: "paid", Total: 25}
	service.applicationEventPublisher.PublishEvent(order, false)
}

// @OnStop
func (service *CheckoutService) Stop() {}
