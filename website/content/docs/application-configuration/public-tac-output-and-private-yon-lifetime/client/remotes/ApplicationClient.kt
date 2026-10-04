@HTTPClient
interface ApplicationClient {
    @GetRequest("/manifest.webmanifest")
    suspend fun load(): Any?
}
