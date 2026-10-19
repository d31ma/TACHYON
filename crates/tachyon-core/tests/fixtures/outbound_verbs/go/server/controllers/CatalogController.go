// @RequestRoute("/catalog")
// @Public
type CatalogController struct {
	// @Needs
	catalog *CatalogClient
}

// @GetRoute
func (controller *CatalogController) GetCatalog() (any, error) {
	found, err := controller.catalog.Find("anvil", "full", "acme")
	if err != nil {
		return nil, err
	}
	saved, err := controller.catalog.Save(map[string]any{"sku": "anvil", "level": 3})
	if err != nil {
		return nil, err
	}
	replaced, err := controller.catalog.Replace(5, map[string]any{"sku": "anvil", "level": 4})
	if err != nil {
		return nil, err
	}
	renamed, err := controller.catalog.Rename(6, map[string]any{"sku": "hammer", "level": 1})
	if err != nil {
		return nil, err
	}
	removed, err := controller.catalog.Remove(7)
	if err != nil {
		return nil, err
	}
	return map[string]any{"found": found, "saved": saved, "replaced": replaced, "renamed": renamed, "removed": removed}, nil
}
