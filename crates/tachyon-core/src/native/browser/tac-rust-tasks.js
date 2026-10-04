/** @typedef {{answer: Promise<string>, cancel: () => void}} RustHttpTicket */
/** @typedef {{start: (text: string) => RustHttpTicket}} RustHttpEndpoint */
/** @typedef {{memory: WebAssembly.Memory, tac_wasm_alloc: (length: number) => number, tac_wasm_dealloc: (pointer: number, length: number) => void, tac_http_settle: (ticket: number, pointer: number, length: number) => void}} RustTaskExports */
class TacRustTasks {
  /** @type {symbol} */
  static symbol = Symbol.for('tachyon.tac.http')
  /** @type {string} */
  static missing = '{"error":{"code":"HTTP_MISSING"}}'
  /** @type {TextEncoder} */
  #encoder = new TextEncoder()
  /** @type {RustTaskExports | undefined} */
  #exports = undefined
  /** @type {Map<number, RustHttpTicket>} */
  #tickets = new Map()
  /** @type {Map<number, (text: string) => void>} */
  #waiting = new Map()

  /** @param {(pointer: number, length: number) => string} read @returns {Record<string, Function>} */
  imports(read) {
    return {
      tac_host_http_start: (ticket, pointer, length) => { this.#start(ticket, read(pointer, length)) },
      tac_host_http_cancel: (ticket) => { this.#cancel(ticket) },
      tac_host_task_done: (task, pointer, length) => { this.#done(task, read(pointer, length)) },
    }
  }

  /** @param {RustTaskExports} exports @returns {void} */
  attach(exports) { this.#exports = exports }

  /** @param {string} text @returns {Promise<string>} */
  answer(text) {
    const task = JSON.parse(text)?.task
    if (!Number.isSafeInteger(task)) return Promise.resolve(text)
    return new Promise((resolve) => { this.#waiting.set(task, resolve) })
  }

  /** @param {number} ticket @param {string} text @returns {void} */
  #start(ticket, text) {
    /** @type {RustHttpEndpoint | undefined} */
    const endpoint = globalThis[TacRustTasks.symbol]
    const started = typeof endpoint?.start === 'function' ? endpoint.start(text) : { answer: Promise.resolve(TacRustTasks.missing), cancel() {} }
    this.#tickets.set(ticket, started)
    started.answer.then((answer) => this.#settle(ticket, answer), () => this.#settle(ticket, TacRustTasks.missing))
  }

  /** @param {number} ticket @returns {void} */
  #cancel(ticket) {
    const started = this.#tickets.get(ticket)
    this.#tickets.delete(ticket)
    started?.cancel()
  }

  /** @param {number} ticket @param {string} answer @returns {void} */
  #settle(ticket, answer) {
    if (!this.#tickets.delete(ticket) || !this.#exports) return
    const bytes = this.#encoder.encode(answer)
    const pointer = this.#exports.tac_wasm_alloc(bytes.length)
    try {
      new Uint8Array(this.#exports.memory.buffer, pointer, bytes.length).set(bytes)
      this.#exports.tac_http_settle(ticket, pointer, bytes.length)
    } finally {
      this.#exports.tac_wasm_dealloc(pointer, bytes.length)
    }
  }

  /** @param {number} task @param {string} text @returns {void} */
  #done(task, text) {
    const resolve = this.#waiting.get(task)
    this.#waiting.delete(task)
    resolve?.(text)
  }
}
