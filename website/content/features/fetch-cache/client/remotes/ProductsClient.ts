@HTTPClient
@CacheSettings({"name":"products","ttlMs":60000,"maxBytes":65536,"maxEntries":16})
export default class ProductsClient {
  @Cached
  @GetRequest("/api/products")
  async list(): Promise<{ items: string[] } | void> {}

  @ClearCache({"method":"list"})
  @PostRequest("/api/products")
  async save(@RequestBody product: BehaviorJsonValue): Promise<BehaviorJsonValue | void> {}
}
