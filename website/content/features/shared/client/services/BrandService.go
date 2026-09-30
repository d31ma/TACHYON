// @Service
type BrandService struct {
	Logo string
}

// @OnStart
func (brand *BrandService) Start() {
	brand.Logo = "/shared/assets/logo.svg"
}
