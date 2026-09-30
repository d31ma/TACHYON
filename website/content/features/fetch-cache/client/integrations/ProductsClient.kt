@CacheSettings({"name":"products","ttlMs":60000,"maxBytes":65536,"maxEntries":16})
@HTTPClient
interface ProductsClient {
    @Cached
    @GetRequest("/api/products")
    fun list(): Any?

    @ClearCache({"method":"list"})
    @PostRequest("/api/products")
    fun save(@RequestBody product: Any?): Any?
}
