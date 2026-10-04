import "example.com/shop/packages/pricing"

// @RequestRoute("/api/price")
// @Public
type PriceController struct {
}

// @GetRoute
func (controller *PriceController) GetPrice() map[string]int {
	return map[string]int{"cents": pricing.WithTax(1000)}
}
