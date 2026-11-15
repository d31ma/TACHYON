@RequestRoute("/me")
@SignedIn
export class MeController {
  /**
   * @param {Identity} caller
   * @returns {string}
   */
  @GetRoute
  getMe(@FromIdentity caller) {
    return caller.user
  }
}
