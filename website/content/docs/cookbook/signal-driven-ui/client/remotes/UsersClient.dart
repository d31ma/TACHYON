@HTTPClient
abstract interface class UsersClient {
  @PostRequest("/users")
  Future<UserSaved> save(@RequestBody String name);
}
