@HTTPClient({"adapter":"products"})
export default class ProductsClient {
  @PostRequest("/api/products")
  async save(product: BehaviorJsonValue): Promise<BehaviorJsonValue> { return {body: product}; }
}