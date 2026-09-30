#[CacheSettings({"name":"products","ttlMs":60000,"maxBytes":65536,"maxEntries":16})]
#[HTTPClient]
trait ProductsClient {
    #[Cached]
    #[GetRequest("/api/products")]
    fn list(&self) -> BehaviorResult<BehaviorValue>;

    #[ClearCache({"method":"list"})]
    #[PostRequest("/api/products")]
    fn save(&self, #[RequestBody] product: BehaviorValue) -> BehaviorResult<BehaviorValue>;
}
