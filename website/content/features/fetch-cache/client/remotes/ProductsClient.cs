[CacheSettings({"name":"products","ttlMs":60000,"maxBytes":65536,"maxEntries":16})]
[HTTPClient]
public interface ProductsClient {
    [Cached]
    [GetRequest("/api/products")]
    System.Threading.Tasks.Task<BehaviorJSON> List();

    [ClearCache({"method":"List"})]
    [PostRequest("/api/products")]
    System.Threading.Tasks.Task<BehaviorJSON> Save([RequestBody] BehaviorJSON product);
}
