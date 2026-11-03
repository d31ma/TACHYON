// @Service
type BasketService struct {
	Count    int
	Label    string
	selected string
	Tabs     []Tab
}

// @OnStart
func (basket *BasketService) Start() {
	basket.Tabs = []Tab{{ID: "a", Name: "A"}}
}

func (basket *BasketService) Increment() {
	basket.Count += 1
}

func (basket *BasketService) Select(id string) {
	basket.selected = id
}
