// @RequestRoute("/catalog")
// @Public
type CatalogController struct {
	// @Needs
	converter *ProductConverter
}

// @GetRoute
func (controller *CatalogController) GetCatalog() (Summary, error) {
	detail := Detail{Name: "detail", Secret: "hidden"}
	value, failure := controller.converter.Map(Product{ID: 7, Name: "Anvil", Detail: detail, Items: []Detail{detail}, Secret: "hidden"})
	if failure != nil {
		return Summary{}, YonResourceError{}
	}
	return value, nil
}
