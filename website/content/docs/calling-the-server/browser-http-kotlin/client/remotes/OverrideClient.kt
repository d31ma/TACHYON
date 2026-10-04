import models.Greeting

@HTTPClient("/api")
interface OverrideClient {
    @GetRequest("/never")
    suspend fun read(): Greeting
}
