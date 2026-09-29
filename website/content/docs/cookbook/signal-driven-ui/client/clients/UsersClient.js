@HTTPClient({"adapter":"users"})
export default class UsersClient {
  /** @returns {Promise<{ name: string }[] | void>} */
  @GetRequest("/users")
  async list() {}
  /** @param {{ name: string }} form */
  @PostRequest("/users")
  async save(@RequestBody form) {}
}