// @RequestRoute("/catalog")
// @Public
type CatalogController struct {
	// @Needs
	catalog *CatalogService
}

// @GetRoute
func (controller *CatalogController) GetCatalog() (any, error) {
	return controller.catalog.Products()
}
