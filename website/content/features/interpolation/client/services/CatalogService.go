// @Service
type CatalogService struct {
	Title   string
	Count   int
	Product Product
}

// @OnStart
func (catalog *CatalogService) Start() {
	catalog.Title = "Catalog"
	catalog.Count = 6
	catalog.Product = Product{Name: "Anvil", Price: "£42.00"}
}
