// @Service
type ProductService struct {
	Summary string
}

// @OnStart
func (product *ProductService) Start() {
	product.Summary = "Not loaded."
}
