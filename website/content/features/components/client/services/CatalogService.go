// @Service
type CatalogService struct {
	Products []Product
}

// @OnStart
func (catalog *CatalogService) Start() {
	catalog.Products = []Product{
		{ID: "anvil", Name: "Anvil", Price: "£42.00"},
		{ID: "rope", Name: "Rope", Price: "£8.00"},
	}
}
