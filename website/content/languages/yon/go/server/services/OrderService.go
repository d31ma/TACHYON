// @Service
type OrderService struct {
	// @Needs
	repository *OrderStore
}

func (service *OrderService) List() []string {
	return service.repository.All()
}
