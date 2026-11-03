import models.Product

@RequestRoute("/products")
@Public
class ProductsController {
  @Needs
  @Named("ProductsService")
  val catalog: ProductsService

  @GetRoute
  fun getProducts(): List<Product> = catalog.list()

  @GetRoute("/{id}")
  fun getProduct(@FromPath id: Long): Product = catalog.find(id)
}
