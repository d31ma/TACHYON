import (
	"io"
	"net/http"
	"strings"
)

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
) any {
	input := map[string]any{"name": product.Name, "level": product.Level}
	_, err := controller.productService.Accept(input)
	return controller.answer(product, err)
}

func (controller *ProductsController) answer(product Product, failure error) any {
	if failure != nil {
		header := http.Header{"Content-Type": {"application/json"}}
		body := io.NopCloser(strings.NewReader(`{"error":"validation"}`))
		return &http.Response{StatusCode: http.StatusBadRequest, Header: header, Body: body}
	}
	return product
}
