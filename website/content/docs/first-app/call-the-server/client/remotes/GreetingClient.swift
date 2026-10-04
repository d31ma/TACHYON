@HTTPClient("/api")
protocol GreetingClient {
  @GetRequest("/greeting")
  func getGreeting() async throws -> Greeting
}
