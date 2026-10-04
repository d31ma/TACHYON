import models.UserSaved

@HTTPClient
interface UsersClient {
    @PostRequest("/users")
    suspend fun save(@RequestBody name: String): UserSaved
}
