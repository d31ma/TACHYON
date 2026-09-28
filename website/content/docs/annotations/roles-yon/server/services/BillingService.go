// @Service
type BillingService struct {
	// @Needs
	repository *BillingStore
	// @Needs
	helper *BillingHelper
}

func (service *BillingService) Total() int {
	subtotal := service.repository.Subtotal()
	return service.helper.Settle(subtotal)
}
