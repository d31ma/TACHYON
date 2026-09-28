import "errors"

// @RequestRoute("/catalog")
// @Public
type CatalogController struct {
	// @Needs
	catalog *CatalogService
}

// @GetRoute
func (controller *CatalogController) GetCatalog() (CacheReport, error) {
	sku := "anvil"
	first, firstErr := controller.catalog.Find(sku)
	repeat, repeatErr := controller.catalog.Find(sku)
	_, writeErr := controller.catalog.Write(sku, 99)
	written, writtenErr := controller.catalog.Find(sku)
	_, invalidateErr := controller.catalog.Invalidate(sku)
	refreshed, refreshedErr := controller.catalog.Find(sku)
	report := CacheReport{First: first, Repeat: repeat, Written: written, Refreshed: refreshed}
	return report, errors.Join(firstErr, repeatErr, writeErr, writtenErr, invalidateErr, refreshedErr)
}
