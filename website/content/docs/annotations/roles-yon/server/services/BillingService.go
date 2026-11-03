// @Service
type BillingService struct {
	// @Needs
	helper *BillingDelegate
}

func (service *BillingService) Total() int {
	return service.helper.Settle(service.subtotal())
}

func (service *BillingService) subtotal() int {
	return 40
}
