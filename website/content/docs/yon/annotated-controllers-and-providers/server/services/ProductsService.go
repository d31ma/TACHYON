// @Service
// @Lifetime("transient")
type ProductsService struct{}

func (service *ProductsService) List() []Product {
	return []Product{service.Find(1)}
}

func (service *ProductsService) Find(id int64) Product {
	return Product{ID: id, Name: "Anvil"}
}
