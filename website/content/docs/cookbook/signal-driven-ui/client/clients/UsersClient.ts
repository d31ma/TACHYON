@HTTPClient({"adapter":"users"})
class UsersClient {
  @GetRequest("/users")
  async list(): Promise<BehaviorJsonValue> { return {}; }
  @PostRequest("/users")
  async save(@RequestBody form: BehaviorJsonValue): Promise<BehaviorJsonValue> {}
}