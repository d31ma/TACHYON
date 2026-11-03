// @Service
type CatalogService struct {
	// @Needs
	catalogClient *CatalogClient
	Count         int
}

func (service *CatalogService) Load() error {
	answer, err := service.catalogClient.All()
	catalog, _ := answer.(map[string]any)
	items, _ := catalog["items"].([]any)
	service.Count = len(items)
	return err
}
