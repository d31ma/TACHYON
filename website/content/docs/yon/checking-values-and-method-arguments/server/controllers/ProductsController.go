// @RequestRoute("/products")
// @Public
type ProductsController struct {
	// @Needs
	productService *ProductService
}

// @PostRoute
func (controller *ProductsController) PostProduct(
	// @FromBody
	product Product,
) (any, error) {
	input := map[string]any{"name": product.Name, "level": product.Level}
	if _, err := controller.productService.Accept(input); err != nil {
		return nil, err
	}
	return product, nil
}
