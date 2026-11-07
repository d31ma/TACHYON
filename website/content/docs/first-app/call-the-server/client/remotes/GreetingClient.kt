import models.Greeting

@HTTPClient("/api")
interface GreetingClient {
  @GetRequest("/greeting")
  suspend fun getGreeting(): Greeting
}
