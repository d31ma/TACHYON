import type { Product } from '/models/Product.ts';

@RequestRoute("/products")
@Public
class ProductsController {
  @GetRoute
  getProducts(): Product[] {
    return [{ name: 'anvil' }]
  }

  @PostRoute
  @ResponseStatus(201)
  postProduct(@FromBody product: Product): Product {
    return product
  }
}
