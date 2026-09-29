@HTTPClient({"adapter":"products"})
export default class ProductsClient {
  /**
   * @param {BehaviorJsonValue} product
   * @returns {Promise<BehaviorJsonValue>}
   */
  @PostRequest("/api/products")
  async save(product) { return {body: product}; }
}