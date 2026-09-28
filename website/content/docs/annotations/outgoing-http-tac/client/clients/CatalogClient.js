@HTTPClient({"cache":"cache-first"})
class CatalogClient {
  @GetRequest("/api/products")
  async all() {}

  /** @param {object} product */
  @PostRequest("/api/products")
  async add(@RequestBody product) {}
}
