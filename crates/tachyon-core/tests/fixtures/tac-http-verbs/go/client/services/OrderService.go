// @Service
type OrderService struct {
	// @Needs
	ordersClient *OrdersClient
}

func (service *OrderService) Replace() (SentRequest, error) {
	return service.ordersClient.Replace("a-7", map[string]any{"item": "hammer"}, "north")
}

func (service *OrderService) Change() (SentRequest, error) {
	return service.ordersClient.Change("a-7", map[string]any{"item": "nails"}, "south")
}

func (service *OrderService) Remove() (SentRequest, error) {
	return service.ordersClient.Remove("a-7", "duplicate")
}
