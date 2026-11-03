// @Service
type CatalogService struct {
	Title string
}

// @OnStart
func (catalog *CatalogService) Start() {
	catalog.Title = "Catalog"
}
