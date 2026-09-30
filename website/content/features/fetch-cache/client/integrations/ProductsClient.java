@CacheSettings({"name":"products","ttlMs":60000,"maxBytes":65536,"maxEntries":16})
@HTTPClient
interface ProductsClient {
    @Cached
    @GetRequest("/api/products")
    Object list();

    @ClearCache({"method":"list"})
    @PostRequest("/api/products")
    Object save(@RequestBody Object product);
}
