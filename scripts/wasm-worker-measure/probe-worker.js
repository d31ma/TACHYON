class ProbeWorker {
  /** @type {Record<string, any>} */
  static service = {};
  static startOperation = 'start';

  /**
   * @param {MessageEvent<{op: string, url: string, name: string, args: unknown[]}>} event
   * @returns {Promise<void>}
   */
  static async answer(event) {
    postMessage(await ProbeWorker._reply(event.data));
  }

  /**
   * @param {{op: string, url: string, name: string, args: unknown[]}} message
   * @returns {Promise<{error?: string, value?: unknown, fields?: Record<string, unknown>}>}
   */
  static async _reply(message) {
    try {
      return await ProbeWorker._handle(message);
    } catch (error) {
      return { error: `${/** @type {Error} */ (error).message}` };
    }
  }

  /**
   * @param {{op: string, url: string, name: string, args: unknown[]}} message
   * @returns {Promise<{value?: unknown, fields: Record<string, unknown>}>}
   */
  static async _handle(message) {
    if (message.op === ProbeWorker.startOperation) return ProbeWorker._start(message.url);
    return ProbeWorker._call(message.name, message.args);
  }

  /**
   * @param {string} url
   * @returns {Promise<{fields: Record<string, unknown>}>}
   */
  static async _start(url) {
    const module = await import(url);
    const service = new module.default({});
    service.__tachyonStarted?.();
    ProbeWorker.service = service;
    return { fields: ProbeWorker._fields() };
  }

  /**
   * @param {string} name
   * @param {unknown[]} args
   * @returns {{value: unknown, fields: Record<string, unknown>}}
   */
  static _call(name, args) {
    const value = ProbeWorker.service[name](...args);
    return { value, fields: ProbeWorker._fields() };
  }

  /** @returns {Record<string, unknown>} */
  static _fields() {
    return Object.fromEntries(Object.keys(ProbeWorker.service).map((name) => [name, ProbeWorker.service[name]]));
  }
}

globalThis.addEventListener('message', (event) => ProbeWorker.answer(event));
