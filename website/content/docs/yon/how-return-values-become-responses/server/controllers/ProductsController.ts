@RequestRoute("/products")
@Public
class ProductsController {
  @GetRoute
  getProducts(): Response {
    return new Response('<main><h1>Products</h1><p>{not-a-template}</p><if>literal</if></main>', {
      status: 200,
      headers: { 'content-type': 'text/html; charset=utf-8' },
    })
  }
}
