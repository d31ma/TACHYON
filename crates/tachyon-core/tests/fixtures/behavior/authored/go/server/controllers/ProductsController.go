// @RequestRoute("/products")
// @Public
type ProductsController struct {
}

// @GetRoute
func (controller *ProductsController) GetProducts() any {
	return CatalogChecks{}.Answer()
}
