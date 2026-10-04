/** @typedef {{answer: Promise<string>, cancel: () => void}} KotlinHttpTicket */
/** @typedef {{start: (text: string) => KotlinHttpTicket}} KotlinHttpEndpoint */
class TacKotlinHttp {
  /** @type {symbol} */
  static symbol = Symbol.for('tachyon.tac.http')
  /** @type {string} */
  static missing = '{"error":{"code":"HTTP_MISSING"}}'
  /** @type {Map<number, KotlinHttpTicket>} */
  _tickets = new Map()

  /** @param {number} ticket @param {string} text @param {(answer: string) => void} settle @returns {void} */
  start(ticket, text, settle) {
    /** @type {KotlinHttpEndpoint | undefined} */
    const endpoint = globalThis[TacKotlinHttp.symbol]
    const started = typeof endpoint?.start === 'function' ? endpoint.start(text) : { answer: Promise.resolve(TacKotlinHttp.missing), cancel() {} }
    this._tickets.set(ticket, started)
    const finish = (answer) => { if (this._tickets.delete(ticket)) settle(answer) }
    started.answer.then(finish, () => finish(TacKotlinHttp.missing))
  }

  /** @param {number} ticket @returns {void} */
  cancel(ticket) {
    const started = this._tickets.get(ticket)
    this._tickets.delete(ticket)
    started?.cancel()
  }
}
