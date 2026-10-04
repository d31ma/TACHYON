// @Service
type CatalogService struct {
	// @Needs
	productsClient *ProductsClient
	Count          int
}

func (catalog *CatalogService) Load() error {
	answer, err := catalog.productsClient.List()
	catalog.Count = catalog.countItems(answer)
	return err
}

func (catalog *CatalogService) Add() error {
	_, err := catalog.productsClient.Save(map[string]any{"name": "anvil"})
	return err
}

func (catalog *CatalogService) countItems(answer BehaviorValue) int {
	object, _ := answer.(map[string]any)
	items, _ := object["items"].([]any)
	return len(items)
}
