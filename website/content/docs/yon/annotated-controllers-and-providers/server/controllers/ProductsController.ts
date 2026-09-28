import type { Product } from '/models/Product.ts'

@RequestRoute("/products")
@Public
class ProductsController {
  @Needs
  @Named("ProductsService")
  catalog: ProductsService;

  @GetRoute
  getProducts(): Product[] { return this.catalog.list(); }

  @GetRoute("/{id}")
  getProduct(@FromPath id: number): Product { return this.catalog.find(id); }
}
