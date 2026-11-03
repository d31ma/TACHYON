// @RequestRoute("/products")
// @Public
type ProductsController struct {
}

// @GetRoute
func (controller *ProductsController) GetProducts() []Product {
	return []Product{{Name: "anvil"}}
}

// @PostRoute
// @ResponseStatus(201)
func (controller *ProductsController) PostProduct(
	// @FromBody
	product Product,
) Product {
	return product
}
