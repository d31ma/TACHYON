@HTTPClient({"adapter":"products"})
class ProductsClient {
  /**
   * @param {object} product
   * @returns {Promise<{ body: object }>}
   */
  @PostRequest("/api/products")
  async save(product) { return {body: product}; }
}