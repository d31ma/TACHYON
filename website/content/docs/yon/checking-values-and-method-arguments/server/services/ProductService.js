/** @import { Product } from '/models/Product.ts' */
@Service
export default class ProductService {
  /**
   * @param {ProductInput} input
   * @returns {Product}
   */
  accept(@Check input) {
    return /** @type {Product} */ (input);
  }
}
