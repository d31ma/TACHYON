// @Service
type CatalogService struct {
	products *ProductsClient
	Count    int
}

// @OnStart
func (catalog *CatalogService) Start() error {
	client, err := NewProductsClient()
	catalog.products = client
	return err
}

func (catalog *CatalogService) Load() error {
	answer, err := catalog.products.List()
	catalog.Count = catalog.countItems(answer)
	return err
}

func (catalog *CatalogService) Add() error {
	_, err := catalog.products.Save(map[string]any{"name": "anvil"})
	return err
}

func (catalog *CatalogService) countItems(answer BehaviorValue) int {
	object, _ := answer.(map[string]any)
	items, _ := object["items"].([]any)
	return len(items)
}
