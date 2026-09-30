// @Service
type BasketService struct {
	Count int64
}

func (basket *BasketService) Doubled() int64 {
	return basket.Count * 2
}

// @OnStart
func (basket *BasketService) Initialize() {
	basket.Count = 6
}
