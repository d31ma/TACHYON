// @Service
type BasketService struct {
	total int
}

// @OnStart
func (basket *BasketService) Start() error {
	basket.total = 1250
	return nil
}

func (basket *BasketService) TotalLabel() string {
	return PriceUtils{}.Label(basket.total)
}
