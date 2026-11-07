// @Service
type OrdersService struct {
	behaviors *BehaviorContext
	Attempts  int
}

// @OnStart
func (orders *OrdersService) Start() error {
	options := NewBehaviorOptions("orders", NewBehaviorSignal())
	context, err := BehaviorsCreate[OrdersSetup](options)
	orders.behaviors = context
	return err
}

// @Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
func (orders *OrdersService) Submit(id BehaviorValue) (BehaviorValue, error) {
	orders.Attempts++
	return orders.reportOf(id), orders.firstTwoAttemptsFail()
}

// @MaxAtOnce({"limit":2,"policy":"block"})
func (orders *OrdersService) Report(id BehaviorValue) (BehaviorValue, error) {
	return orders.reportOf(id), nil
}

func (orders *OrdersService) reportOf(id BehaviorValue) map[string]any {
	return map[string]any{"id": id, "attempts": orders.Attempts}
}

func (OrdersService) firstWorkingAttempt() int { return 3 }

func (orders *OrdersService) firstTwoAttemptsFail() error {
	if orders.Attempts < orders.firstWorkingAttempt() {
		return &BehaviorError{Code: "adapter", Message: "the checkout API is unavailable"}
	}
	return nil
}
