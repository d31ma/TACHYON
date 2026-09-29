@HTTPClient({"adapter":"api"})
export default class CatalogClient {
  /** @returns {Promise<BehaviorJsonValue>} */
  @GetRequest("/api/products")
  async all() { return {} }
}