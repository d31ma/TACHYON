@HTTPClient("/api")
abstract interface class GreetingClient {
  @GetRequest("/greeting")
  Future<Greeting> getGreeting();
}
