@RequestRoute("/products")
class ProductsController {
  @GetRoute
  getProducts() {
    return new Response('<main><h1>Products</h1><p>{ordinary text}</p></main>', {
      status: 200,
      headers: { 'content-type': 'text/html; charset=utf-8' },
    })
  }
}