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

func (service *CatalogService) All() (BehaviorValue, error) {
	return service.catalog.All()
}
