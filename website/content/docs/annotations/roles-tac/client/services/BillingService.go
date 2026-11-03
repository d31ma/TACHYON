// @Service
type BillingService struct {
	Total int
}

// @OnStart
func (billing *BillingService) Start() {
	billing.Total = 42
}
