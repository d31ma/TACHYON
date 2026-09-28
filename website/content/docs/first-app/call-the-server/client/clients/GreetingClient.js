@HTTPClient
class GreetingClient {
  @GetRequest("/api/greeting")
  async getGreeting() {}
}
