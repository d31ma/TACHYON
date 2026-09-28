@HTTPClient({"adapter":"api"})
class CatalogClient {
  /** @returns {Promise<unknown>} */
  @GetRequest("/api/products")
  async all() { return {} }
}