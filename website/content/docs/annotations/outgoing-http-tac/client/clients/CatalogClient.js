@HTTPClient({"cache":"cache-first"})
export default class CatalogClient {
  /** @returns {Promise<{ items: string[] } | void>} */
  @GetRequest("/api/products")
  async all() {}

  /** @param {object} product */
  @PostRequest("/api/products")
  async add(@RequestBody product) {}
}
