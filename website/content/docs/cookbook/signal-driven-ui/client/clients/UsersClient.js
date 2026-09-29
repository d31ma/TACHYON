@HTTPClient({"adapter":"users"})
export default class UsersClient {
  /** @returns {Promise<{ name: string }[]>} */
  @GetRequest("/users")
  async list() { return []; }
  /** @param {{ name: string }} form */
  @PostRequest("/users")
  async save(@RequestBody form) {}
}