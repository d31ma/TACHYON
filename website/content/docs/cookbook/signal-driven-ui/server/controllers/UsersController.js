@RequestRoute("/users")
@Public
export default class UsersController {
  /**
   * @param {string} name
   * @returns {{ name: string }}
   */
  @PostRoute
  postUser(@FromBody name) {
    return { name }
  }
}
