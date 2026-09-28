@RequestRoute("/products")
@Public
class ProductsController {
    @GetRoute
    Object getProducts() {
        return ProductChecksHelper.run()
    }
}
