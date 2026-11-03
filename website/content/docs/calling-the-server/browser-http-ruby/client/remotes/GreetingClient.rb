#[HTTPClient("/api")]
#[CacheSettings({"name":"greetings","ttlMs":60000,"maxBytes":4096,"maxEntries":8})]
class GreetingClient
  # @return [Greeting]
  #[Cached]
  #[GetRequest("/greeting")]
  def read
  end

  # @return [Greeting]
  #[ClearCache({"method":"read"})]
  #[PostRequest("/clear")]
  def clear
  end
end
