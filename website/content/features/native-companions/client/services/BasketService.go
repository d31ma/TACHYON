type BasketService struct {
	Count int
	Label string
}

// @OnStart
func (basket *BasketService) Start() {
	basket.Count = 6
	basket.Label = "Go"
}

func (basket *BasketService) Doubled() int {
	return basket.Count * 2
}
