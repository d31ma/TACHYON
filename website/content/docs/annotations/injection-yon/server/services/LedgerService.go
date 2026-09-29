// @Service
type LedgerService struct {
	// @Needs
	amounts *LedgerAmountsHelper
}

func (service *LedgerService) Subtotal() int {
	return service.amounts.Subtotal()
}
