// @RequestRoute("/catalog")
// @Public
type CatalogController struct {
	// @Needs
	converter *ProductConverter
}

// @GetRoute
func (controller *CatalogController) GetCatalog() (CatalogSummary, error) {
	product := map[string]any{
		"detail": map[string]any{"id": 7},
		"name":   "Anvil",
		"extra":  "dropped",
	}
	summary, err := controller.converter.ToSummary(product)
	return CatalogSummary{Summary: summary}, controller.asResourceError(err)
}

func (controller *CatalogController) asResourceError(failure error) error {
	if failure != nil {
		return YonResourceError{}
	}
	return nil
}
