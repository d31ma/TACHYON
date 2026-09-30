import (
	"io"
	"net/http"
	"strings"
)

// @RequestRoute("/products")
// @Public
type ProductsController struct {
}

// @GetRoute
func (controller *ProductsController) GetProducts() *http.Response {
	header := http.Header{"Content-Type": {"text/html; charset=utf-8"}}
	body := io.NopCloser(strings.NewReader(`<main><h1>Products</h1><p>{not-a-template}</p><if>literal</if></main>`))
	return &http.Response{StatusCode: http.StatusOK, Header: header, Body: body}
}
