import models.Product

@RequestRoute("/products")
@Public
class ProductsController {
    @Needs
    val productService: ProductService

    @PostRoute
    fun postProduct(@FromBody product: Product): Product {
        productService.accept(mapOf("name" to product.name, "level" to product.level))
        return product
    }
}
