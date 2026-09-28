import type { Product } from '/models/Product.ts';

@RequestRoute("/products")
@Public
class ProductsController {
  @Needs
  readonly productService!: ProductService;

  @PostRoute
  postProduct(@FromBody product: Product): Product {
    return this.productService.accept(product);
  }
}
