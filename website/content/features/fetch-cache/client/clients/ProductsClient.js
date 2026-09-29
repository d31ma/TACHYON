@HTTPClient({"adapter":"products"})
export default class ProductsClient {
  /**
   * @param {BehaviorJsonValue} product
   * @returns {Promise<BehaviorJsonValue | void>}
   */
  @PostRequest("/api/products")
  async save(@RequestBody product) {}
}
