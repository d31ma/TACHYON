/** @typedef {{path: string, method: string, credentials: RequestCredentials, authorization: boolean}} TaskModelRequest */

export class DotnetTaskModelRequests {
  /** @param {import('playwright').Page} page @param {string} prefix @param {number} limit @returns {Promise<void>} */
  static async observe(page, prefix, limit) {
    await DotnetTaskModelRequests.#install(page, { prefix, limit })
  }

  /** @param {import('playwright').Page} page @returns {Promise<TaskModelRequest[]>} */
  static async read(page) {
    return page.evaluate(() => Reflect.get(globalThis, '__taskDtoRequests'))
  }

  /** @param {import('playwright').Page} page @param {{prefix: string, limit: number}} bounds @returns {Promise<void>} */
  static async #install(page, bounds) {
    await page.addInitScript(({ prefix, limit }) => {
      const original = globalThis.fetch.bind(globalThis)
      /** @type {TaskModelRequest[]} */
      const requests = []
      Reflect.set(globalThis, '__taskDtoRequests', requests)
      globalThis.fetch = (input, options) => {
        const source = input instanceof Request ? input : undefined
        const url = new URL(source?.url ?? String(input), location.href)
        if (url.pathname.startsWith(prefix) && requests.length < limit) {
          const headers = new Headers(options?.headers ?? source?.headers)
          requests.push({ path: url.pathname, method: options?.method ?? source?.method ?? 'GET', credentials: options?.credentials ?? source?.credentials ?? 'same-origin', authorization: headers.has('authorization') })
        }
        return original(input, options)
      }
    }, bounds)
  }
}
