@HTTPClient("/api")
export default class CatalogClient {
  @GetRequest("/products/{id}")
  async find(@PathVariable("id") id: string, @RequestParam("view") view: string, @RequestHeader("X-Tenant") tenant: string): Promise<BehaviorJsonValue | void> {}

  @PostRequest("/products")
  async save(@RequestBody product: BehaviorJsonValue): Promise<BehaviorJsonValue | void> {}

  @PutRequest("/products/{id}")
  async replace(@PathVariable("id") id: number, @RequestBody product: BehaviorJsonValue): Promise<BehaviorJsonValue | void> {}

  @PatchRequest("/products/{id}")
  async rename(@PathVariable("id") id: number, @RequestBody product: BehaviorJsonValue): Promise<BehaviorJsonValue | void> {}

  @DeleteRequest("/products/{id}")
  async remove(@PathVariable("id") id: number): Promise<BehaviorJsonValue | void> {}
}
