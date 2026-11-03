/** @import { UserSaved } from '/models/UserSaved.ts' */
@HTTPClient
export default class UsersClient {
  /**
   * @param {string} name
   * @returns {Promise<UserSaved | void>}
   */
  @PostRequest("/users")
  async save(@RequestBody name) {}
}
