/** @import { Product } from '/models/Product.ts' */
@RequestRoute("/products")
@Public
class ProductsController {
  /** @type {ProductService} */
  @Needs
  productService;

  /**
   * @param {Product} product
   * @returns {Product}
   */
  @PostRoute
  postProduct(@FromBody product) {
    return this.productService.accept(product);
  }
}
