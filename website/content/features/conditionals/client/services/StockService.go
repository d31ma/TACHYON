// @Service
type StockService struct {
	Products   []Product
	Restocking bool
}

// @OnStart
func (stock *StockService) Start() {
	stock.Products = []Product{}
	stock.Restocking = true
}
