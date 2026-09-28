@HTTPClient({"adapter":"products"})
class ProductsClient {
  @PostRequest("/api/products")
  async save(product: BehaviorJsonValue): Promise<BehaviorJsonValue> { return {body: product}; }
}