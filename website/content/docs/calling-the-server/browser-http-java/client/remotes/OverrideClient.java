import models.Greeting;

@HTTPClient("/api")
interface OverrideClient {
    @GetRequest("/never")
    Greeting read();
}
