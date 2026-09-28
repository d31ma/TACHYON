type MoneyService struct {
	Amount int64
}

// @OnStart
func (service *MoneyService) Start() {
	service.Amount = 42
}
