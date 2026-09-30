@HTTPClient
@CacheSettings({"name":"catalog","ttlMs":60000,"maxBytes":65536,"maxEntries":16})
export default class CatalogClient {
  /** @returns {Promise<{ items: string[] } | void>} */
  @Cached
  @GetRequest("/api/products")
  async all() {}

  /** @param {object} product */
  @PostRequest("/api/products")
  async add(@RequestBody product) {}
}
