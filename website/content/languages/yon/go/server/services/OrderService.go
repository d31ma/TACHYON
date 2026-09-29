// @Service
type OrderService struct{}

func (service *OrderService) List() []string {
	return []string{"anvil"}
}
