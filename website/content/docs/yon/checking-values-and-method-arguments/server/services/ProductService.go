// @Service
type ProductService struct{}

func (service *ProductService) Accept(
	// @Check(ProductInput)
	input BehaviorValue,
) (BehaviorValue, error) {
	return input, nil
}
