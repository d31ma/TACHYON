// @HTTPClient("/api")
// @CacheSettings({"name":"greetings","ttlMs":60000,"maxBytes":4096,"maxEntries":8})
type GreetingClient interface {
	// @Cached
	// @GetRequest("/greeting")
	Read() (Greeting, error)

	// @ClearCache({"method":"Read"})
	// @PostRequest("/clear")
	Clear() (Greeting, error)
}
