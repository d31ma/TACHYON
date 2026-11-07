@HTTPClient({"adapter":"users","setup":"UsersSetup"})
abstract interface class UsersClient {
  @PostRequest("/users")
  Future<UserSaved> save(@RequestBody String name);

  @GetRequest("/users")
  Future<List<UserSaved>> all();
}
