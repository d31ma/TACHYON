import models.UserSaved

@HTTPClient({"adapter":"users","setup":"UsersSetup"})
interface UsersClient {
    @PostRequest("/users")
    suspend fun save(@RequestBody name: String): UserSaved

    @GetRequest("/users")
    fun all(): Any?
}
