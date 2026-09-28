@IdentityProvider
export class AppIdentityProvider {
  /** @type {Map<string, { user: string, roles: string[] }>} */
  static _SESSIONS = new Map([
    ['ada-token', { user: 'ada', roles: ['Admin'] }],
    ['bob-token', { user: 'bob', roles: [] }],
  ])

  /**
   * @param {Request} request
   * @returns {{ user: string, roles: string[] } | null}
   */
  identify(request) {
    return this.#find(String(request.headers.get('authorization')))
  }

  /**
   * @param {string} token
   * @returns {{ user: string, roles: string[] } | null}
   */
  #find(token) {
    return AppIdentityProvider._SESSIONS.get(token) ?? null
  }
}
