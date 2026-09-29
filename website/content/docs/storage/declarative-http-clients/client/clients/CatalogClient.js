@HTTPClient({"adapter":"api"})
export default class CatalogClient {
  /** @returns {Promise<BehaviorJsonValue | void>} */
  @GetRequest("/api/products")
  async all() {}
}