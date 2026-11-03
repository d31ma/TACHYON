// @RequestRoute("/products")
// @Public
type ProductsController struct {
	// @Needs
	// @Named("ProductsService")
	catalog *ProductsService
}

// @GetRoute
func (controller *ProductsController) GetProducts() []Product {
	return controller.catalog.List()
}

// @GetRoute("/{id}")
func (controller *ProductsController) GetProduct(
	// @FromPath
	id int64,
) Product {
	return controller.catalog.Find(id)
}
