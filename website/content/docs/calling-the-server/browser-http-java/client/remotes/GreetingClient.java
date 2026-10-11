import models.Greeting;

@HTTPClient("/api")
@CacheSettings({"name":"greetings","ttlMs":60000,"maxBytes":4096,"maxEntries":8})
interface GreetingClient {
    @Cached
    @GetRequest("/greeting")
    Greeting read();

    @ClearCache({"method":"read"})
    @PostRequest("/clear")
    Greeting clear();
}
