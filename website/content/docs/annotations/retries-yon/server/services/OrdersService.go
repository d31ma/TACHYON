// @Service
type OrdersService struct {
	behaviors *BehaviorContext
	attempts  float64
}

// @OnStart
func (service *OrdersService) Open() error {
	options := NewBehaviorOptions("orders", NewBehaviorSignal())
	context, err := BehaviorsCreate[OrdersSetup](options)
	service.behaviors = context
	return err
}

// @Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
func (service *OrdersService) Submit(id BehaviorValue) (BehaviorValue, error) {
	service.attempts++
	return service.reportOf(id), service.firstTwoAttemptsFail()
}

// @MaxAtOnce({"limit":2,"policy":"block"})
func (service *OrdersService) Report(id BehaviorValue) (BehaviorValue, error) {
	return service.reportOf(id), nil
}

func (service *OrdersService) reportOf(id BehaviorValue) map[string]any {
	return map[string]any{"id": id, "attempts": service.attempts}
}

func (OrdersService) firstWorkingAttempt() float64 { return 3 }

func (service *OrdersService) firstTwoAttemptsFail() error {
	if service.attempts < service.firstWorkingAttempt() {
		return &BehaviorError{Code: "adapter", Message: "the payment gateway is unavailable"}
	}
	return nil
}
