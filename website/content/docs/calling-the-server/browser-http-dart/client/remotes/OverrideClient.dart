@HTTPClient("/api")
abstract interface class OverrideClient {
  @GetRequest("/never")
  Future<Greeting> read();
}
