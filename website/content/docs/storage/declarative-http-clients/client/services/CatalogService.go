// @Service
type CatalogService struct {
	catalog  *CatalogClient
	Products []string
	Error    string
}

// @OnStart
func (service *CatalogService) Start() error {
	client, err := NewCatalogClient()
	service.catalog = client
	return err
}

func (service *CatalogService) Load() {
	service.show(service.fetch())
}

func (service *CatalogService) fetch() []any {
	answer, _ := service.catalog.All()
	items, _ := answer.([]any)
	return items
}

func (service *CatalogService) show(items []any) {
	if items == nil {
		service.showFailure()
		return
	}
	service.Products = service.names(items)
	service.Error = ""
}

func (service *CatalogService) names(items []any) []string {
	found := []string{}
	for _, item := range items {
		name, _ := item.(string)
		found = append(found, name)
	}
	return found
}

func (service *CatalogService) showFailure() {
	service.Error = "Products could not be loaded. Please retry."
}
