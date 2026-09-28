@HTTPClient({"cache":"cache-first"})
class CatalogClient {
  @GetRequest("/api/products")
  async all(): Promise<BehaviorJsonValue> {}

  @PostRequest("/api/products")
  async add(@RequestBody product: BehaviorJsonValue): Promise<BehaviorJsonValue> {}
}
