@HTTPClient
export default class CatalogClient {
  /**
   * @param {number} page
   * @returns {Promise<BehaviorJsonValue | void>}
   */
  @GetRequest("/products")
  async all(@RequestParam("page") page) {}

  /**
   * @param {BehaviorJsonValue} product
   * @returns {Promise<BehaviorJsonValue | void>}
   */
  @PostRequest("/products")
  async add(@RequestBody product) {}
}
