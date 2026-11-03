import models.Greeting

@HTTPClient("/api")
@CacheSettings({"name":"greetings","ttlMs":60000,"maxBytes":4096,"maxEntries":8})
interface GreetingClient {
    @Cached
    @GetRequest("/greeting")
    suspend fun read(): Greeting

    @ClearCache({"method":"read"})
    @PostRequest("/clear")
    suspend fun clear(): Greeting
}
