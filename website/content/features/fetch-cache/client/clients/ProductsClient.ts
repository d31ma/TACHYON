@HTTPClient({"adapter":"products"})
export default class ProductsClient {
  @PostRequest("/api/products")
  async save(@RequestBody product: BehaviorJsonValue): Promise<BehaviorJsonValue> {}
}
