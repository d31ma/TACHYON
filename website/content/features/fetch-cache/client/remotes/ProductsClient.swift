@CacheSettings({"name":"products","ttlMs":60000,"maxBytes":65536,"maxEntries":16})
@HTTPClient
protocol ProductsClient {
    @Cached
    @GetRequest("/api/products")
    func list() async throws -> BehaviorJSON

    @ClearCache({"method":"list"})
    @PostRequest("/api/products")
    func save(@RequestBody product: BehaviorJSON) async throws -> BehaviorJSON
}
