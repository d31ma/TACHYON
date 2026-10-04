@HTTPClient
interface UsersClient {
    @PostRequest("/users")
    Object save(@RequestBody Object name);
}
