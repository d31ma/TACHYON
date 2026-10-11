/** @typedef {{answer: Promise<string>, cancel: () => void}} HttpTicket */
/** @typedef {{start: (text: string) => HttpTicket}} HttpEndpoint */
class TacTicketHttp {
  /** @type {symbol} */
  static symbol = Symbol.for('tachyon.tac.http')
  /** @type {string} */
  static missing = '{"error":{"code":"HTTP_MISSING"}}'
  /** @type {Map<number, HttpTicket>} */
  _tickets = new Map()

  /** @param {number} ticket @param {string} text @param {(answer: string) => void} settle @returns {void} */
  start(ticket, text, settle) {
    /** @type {HttpEndpoint | undefined} */
    const endpoint = globalThis[TacTicketHttp.symbol]
    const started = typeof endpoint?.start === 'function' ? endpoint.start(text) : { answer: Promise.resolve(TacTicketHttp.missing), cancel() {} }
    this._tickets.set(ticket, started)
    const finish = (answer) => { if (this._tickets.delete(ticket)) settle(answer) }
    started.answer.then(finish, () => finish(TacTicketHttp.missing))
  }

  /** @param {number} ticket @returns {void} */
  cancel(ticket) {
    const started = this._tickets.get(ticket)
    this._tickets.delete(ticket)
    started?.cancel()
  }
}
