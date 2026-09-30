// @Service
type CatalogService struct {
	catalog *CatalogClient
	Count   int
}

// @OnStart
func (service *CatalogService) Start() error {
	client, err := NewCatalogClient()
	service.catalog = client
	return err
}

func (service *CatalogService) Load() error {
	answer, err := service.catalog.All()
	catalog, _ := answer.(map[string]any)
	items, _ := catalog["items"].([]any)
	service.Count = len(items)
	return err
}
