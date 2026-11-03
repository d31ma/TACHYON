import models.Product

@RequestRoute("/products")
@Public
class ProductsController {
    @GetRoute
    fun getProducts(): List<Product> = listOf(Product("anvil"))

    @PostRoute
    @ResponseStatus(201)
    fun postProduct(@FromBody product: Product): Product = product
}
