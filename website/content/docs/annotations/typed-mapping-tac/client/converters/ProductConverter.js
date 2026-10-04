@Converter
export default class ProductConverter {
  /**
   * @param {Product} product
   * @returns {Promise<Summary | void>}
   */
  @Convert({"fields":[{"target":"kind","constant":"product"}]})
  async map(product) {}

  /**
   * @param {Product} product
   * @returns {Promise<Product | void>}
   */
  async copy(product) {}
}
