@HTTPClient({"adapter":"api"})
class CatalogClient {
  @GetRequest("/api/products")
  async all(): Promise<BehaviorJsonValue> { return {} }
}