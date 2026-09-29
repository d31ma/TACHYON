@HTTPClient({"adapter":"api"})
export default class CatalogClient {
  @GetRequest("/api/products")
  async all(): Promise<BehaviorJsonValue> { return {} }
}