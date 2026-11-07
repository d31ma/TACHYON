import models.Product

@RequestRoute("/products")
@Public
class ProductsController {
    @Needs
    final ProductService productService

    @PostRoute
    Product postProduct(@FromBody Product product) {
        productService.accept(java.util.Map.of("name", product.name(), "level", product.level()))
        return product
    }
}
