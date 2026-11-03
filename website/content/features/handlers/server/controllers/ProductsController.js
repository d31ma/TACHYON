/** @import { Product } from '/models/Product.ts' */
@RequestRoute("/products")
@Public
class ProductsController {
  /** @returns {Product[]} */
  @GetRoute
  getProducts() {
    return [{ name: 'anvil' }]
  }

  /**
   * @param {Product} product
   * @returns {Product}
   */
  @PostRoute
  @ResponseStatus(201)
  postProduct(@FromBody product) {
    return product
  }
}
