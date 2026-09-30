// @RequestRoute("/catalog")
// @Public
type CatalogController struct {
	// @Needs
	converter *ProductConverter
}

// @GetRoute
func (controller *CatalogController) GetCatalog() (Summary, error) {
	detail := Detail{Name: "detail", Secret: "detail-private"}
	value, failure := controller.converter.Map(Product{ID: 7, Name: "Anvil", Detail: detail, Items: []Detail{detail}, Secret: "product-private"})
	return value, controller.asResourceError(failure)
}

func (controller *CatalogController) asResourceError(failure error) error {
	if failure != nil {
		return YonResourceError{}
	}
	return nil
}
