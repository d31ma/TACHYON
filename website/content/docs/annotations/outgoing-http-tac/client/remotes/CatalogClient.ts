@HTTPClient
@CacheSettings({"name":"catalog","ttlMs":60000,"maxBytes":65536,"maxEntries":16})
export default class CatalogClient {
  @Cached
  @GetRequest("/api/products")
  async all(): Promise<{ items: string[] } | void> {}

  @PostRequest("/api/products")
  async add(@RequestBody product: BehaviorJsonValue): Promise<BehaviorJsonValue | void> {}
}
