/** @import { Product } from '/models/Product.ts' */
@RequestRoute("/products")
@Public
class ProductsController {
  /** @type {ProductsService} */
  @Needs
  @Named("ProductsService")
  catalog;

  /** @returns {Product[]} */
  @GetRoute
  getProducts() { return this.catalog.list(); }

  /**
   * @param {number} id
   * @returns {Product}
   */
  @GetRoute("/{id}")
  getProduct(@FromPath id) { return this.catalog.find(id); }
}
