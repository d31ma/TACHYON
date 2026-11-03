import models.Greeting;

@HTTPClient("/api")
interface GreetingClient {
    @GetRequest("/greeting")
    Greeting getGreeting();
}
