// @Service
type LedgerService struct {
	// @Needs
	amounts *LedgerAmountsDelegate
}

func (service *LedgerService) Subtotal() int {
	return service.amounts.Subtotal()
}
