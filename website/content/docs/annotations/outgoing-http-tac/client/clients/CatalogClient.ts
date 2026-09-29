@HTTPClient({"cache":"cache-first"})
export default class CatalogClient {
  @GetRequest("/api/products")
  async all(): Promise<{ items: string[] } | void> {}

  @PostRequest("/api/products")
  async add(@RequestBody product: BehaviorJsonValue): Promise<BehaviorJsonValue | void> {}
}
