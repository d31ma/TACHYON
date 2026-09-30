@CacheSettings({"name":"products","ttlMs":60000,"maxBytes":65536,"maxEntries":16})
@HTTPClient
abstract interface class ProductsClient {
  @Cached
  @GetRequest("/api/products")
  Future<Object?> list();

  @ClearCache({"method":"list"})
  @PostRequest("/api/products")
  Future<Object?> save(@RequestBody Object? product);
}
