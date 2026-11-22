class TacStreamReport {
  /** @type {number} */
  static #limit = 393216
  /** @type {string} */
  static #tooLargeMessage = 'Application event exceeds 393216 bytes; send less data in one event.'
  /** @type {Set<string>} */
  static #reported = new Set()

  /** @param {unknown} error @param {string} path @param {string} event @param {string} data */
  static failed(error, path, event, data) {
    TacStreamReport.#reportOnce(TacStreamReport.#isTooLarge(error), path, event, data)
  }

  /** @param {unknown} error @returns {boolean} */
  static #isTooLarge(error) {
    return error instanceof RangeError && error.message === TacStreamReport.#tooLargeMessage
  }

  /** @param {boolean} tooLarge @param {string} path @param {string} event @param {string} data */
  static #reportOnce(tooLarge, path, event, data) {
    const key = `${path} ${event}`
    if (!tooLarge || TacStreamReport.#reported.has(key)) return
    TacStreamReport.#reported.add(key)
    console.error(TacStreamReport.#text(path, event, new Blob([data]).size))
  }

  /** @param {string} path @param {string} event @param {number} size @returns {string} */
  static #text(path, event, size) {
    return `Tac dropped a message on the stream ${path}. Its '${event}' event has ${size} bytes, and the limit is ${TacStreamReport.#limit} bytes. Tac kept the stream open and does not report this again for this stream and event. Send smaller messages, for example fewer items or a shorter time window.`
  }
}
