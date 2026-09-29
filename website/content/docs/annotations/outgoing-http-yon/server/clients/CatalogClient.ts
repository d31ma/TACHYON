@HTTPClient
export default class CatalogClient {
  @GetRequest("/products")
  async all(@RequestParam("page") page: number): Promise<BehaviorJsonValue | void> {}

  @PostRequest("/products")
  async add(@RequestBody product: BehaviorJsonValue): Promise<BehaviorJsonValue | void> {}
}
