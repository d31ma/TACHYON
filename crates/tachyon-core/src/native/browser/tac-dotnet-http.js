/** @typedef {{answer: Promise<string>, cancel: () => void}} DotnetHttpTicket */
/** @typedef {{start: (text: string) => DotnetHttpTicket}} DotnetHttpEndpoint */
class TacDotnetHttp {
  /** @type {number} */
  static limit = 64
  /** @type {symbol} */
  static symbol = Symbol.for('tachyon.tac.http')
  /** @type {Map<string, DotnetHttpTicket>} */
  #tickets = new Map()

  /** @param {string} text @returns {string} */
  start(text) { return this.#begin(text) }

  /** @param {string} id @returns {Promise<string>} */
  answer(id) { return this.#wait(id, this.#find(id)) }

  /** @param {string} id @returns {void} */
  cancel(id) { this.#tickets.get(id)?.cancel() }

  /** @param {string} text @returns {string} */
  #begin(text) {
    /** @type {DotnetHttpEndpoint | undefined} */
    const endpoint = globalThis[TacDotnetHttp.symbol]
    const code = this.#refusal(endpoint)
    return this.#reserve(text, endpoint, code)
  }

  /** @param {DotnetHttpEndpoint | undefined} endpoint @returns {string | undefined} */
  #refusal(endpoint) {
    switch (true) {
      case this.#tickets.size >= TacDotnetHttp.limit: return 'HTTP_CAPACITY'
      case typeof endpoint?.start !== 'function': return 'HTTP_MISSING'
      default: return undefined
    }
  }

  /** @param {string} text @param {DotnetHttpEndpoint | undefined} endpoint @param {string | undefined} code @returns {string} */
  #reserve(text, endpoint, code) {
    if (code) return JSON.stringify({ error: { code } })
    const id = crypto.randomUUID()
    this.#tickets.set(id, endpoint.start(text))
    return id
  }

  /** @param {string} id @returns {DotnetHttpTicket} */
  #find(id) {
    return this.#tickets.get(id) ?? { answer: Promise.resolve('{"error":{"code":"HTTP_MISSING"}}'), cancel() {} }
  }

  /** @param {string} id @param {DotnetHttpTicket} ticket @returns {Promise<string>} */
  async #wait(id, ticket) {
    try { return await ticket.answer }
    finally { this.#tickets.delete(id) }
  }
}
