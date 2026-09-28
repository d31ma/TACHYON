// @RequestRoute("/catalog")
// @Public
type CatalogController struct {
	// @Needs
	catalog *CatalogStore
}

// @GetRoute
func (controller *CatalogController) GetCatalog() (any, error) {
	return controller.catalog.Find()
}
