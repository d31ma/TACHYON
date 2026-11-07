@HTTPClient("/api")
@CacheSettings({"name":"greetings","ttlMs":60000,"maxBytes":4096,"maxEntries":8})
abstract interface class GreetingClient {
  @Cached
  @GetRequest("/greeting")
  Future<Greeting> read();

  @ClearCache({"method":"read"})
  @PostRequest("/clear")
  Future<Greeting> clear();
}
