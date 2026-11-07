#[CacheSettings({"name":"products","ttlMs":60000,"maxBytes":65536,"maxEntries":16})]
#[HTTPClient]
class ProductsClient
  # @return [Hash]
  #[Cached]
  #[GetRequest("/api/products")]
  def list
  end

  # @param product [Hash]
  # @return [Hash]
  #[ClearCache({"method":"list"})]
  #[PostRequest("/api/products")]
  def save(
    #[RequestBody]
    product
  )
  end
end
