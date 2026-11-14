@IdentityProvider
export class AppIdentityProvider {
  /** @type {Map<string, { user: string, roles: string[] }>} */
  static _SESSIONS = new Map([
    ['ada-token', { user: 'ada', roles: ['Admin'] }],
    ['bob-token', { user: 'bob', roles: [] }],
  ])

  static _BROKEN_TOKEN = 'broken-token'

  /**
   * @param {Request} request
   * @returns {{ user: string, roles: string[] } | null}
   */
  identify(request) {
    const token = String(request.headers.get('authorization'))
    this._refuseBroken(token)
    return this._find(token)
  }

  /** @param {string} token */
  _refuseBroken(token) {
    if (token === AppIdentityProvider._BROKEN_TOKEN) throw new Error('the session store is down')
  }

  /**
   * @param {string} token
   * @returns {{ user: string, roles: string[] } | null}
   */
  _find(token) {
    return AppIdentityProvider._SESSIONS.get(token) ?? null
  }
}
