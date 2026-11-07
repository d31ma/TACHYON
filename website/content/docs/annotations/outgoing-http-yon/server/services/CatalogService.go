// @Service
type CatalogService struct {
	// @Needs
	catalog *CatalogClient
}

func (service *CatalogService) Products() (BehaviorValue, error) {
	return service.catalog.All(1)
}
