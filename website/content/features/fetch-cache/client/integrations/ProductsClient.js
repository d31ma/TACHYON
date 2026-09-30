@HTTPClient
@CacheSettings({"name":"products","ttlMs":60000,"maxBytes":65536,"maxEntries":16})
export default class ProductsClient {
  /** @returns {Promise<{ items: string[] } | void>} */
  @Cached
  @GetRequest("/api/products")
  async list() {}

  /**
   * @param {BehaviorJsonValue} product
   * @returns {Promise<BehaviorJsonValue | void>}
   */
  @ClearCache({"method":"list"})
  @PostRequest("/api/products")
  async save(@RequestBody product) {}
}
