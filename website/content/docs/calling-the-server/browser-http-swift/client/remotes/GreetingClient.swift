@HTTPClient("/api")
@CacheSettings({"name":"greetings","ttlMs":60000,"maxBytes":4096,"maxEntries":8})
protocol GreetingClient {
    @Cached
    @GetRequest("/greeting")
    func read() async throws -> Greeting

    @ClearCache({"method":"read"})
    @PostRequest("/clear")
    func clear() async throws
}
