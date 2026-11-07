use crate::models::Greeting;

#[HTTPClient("/api")]
#[CacheSettings({"name":"greetings","ttlMs":60000,"maxBytes":4096,"maxEntries":8})]
trait GreetingClient {
    #[Cached]
    #[GetRequest("/greeting")]
    async fn read(&self) -> BehaviorResult<Greeting>;

    #[ClearCache({"method":"read"})]
    #[PostRequest("/clear")]
    async fn clear(&self) -> BehaviorResult<Greeting>;
}
