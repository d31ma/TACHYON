// @Service
type LedgerService struct {
	// @Needs
	repository *LedgerStore
}

func (service *LedgerService) Subtotal() int {
	return service.repository.Subtotal()
}
