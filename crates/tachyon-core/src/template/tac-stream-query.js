class TacStreamQuery {
  /** @type {string[]} */
  static #plainTypes = ['string', 'number', 'boolean']
  /** @type {RegExp} */
  static #pathPart = /\{(\w+)\}/g

  /** @param {string} template @param {unknown} value @returns {string} */
  static address(template, value) {
    const fields = TacStreamQuery.#fields(value)
    const path = template.replace(TacStreamQuery.#pathPart, (_, name) => TacStreamQuery.#take(fields, name))
    return TacStreamQuery.#join(path, TacStreamQuery.#query(TacStreamQuery.#present(fields)))
  }

  /** @param {unknown} value @returns {Record<string, unknown>} */
  static #fields(value) {
    if (value?.constructor !== Object) throw new TypeError('Stream parameters must be a model object.')
    return { ...value }
  }

  /** @param {Record<string, unknown>} fields @param {string} name @returns {string} */
  static #take(fields, name) {
    const item = fields[name]
    delete fields[name]
    return encodeURIComponent(TacStreamQuery.#plain(item))
  }

  /** @param {unknown} item @returns {string} */
  static #plain(item) {
    if (!TacStreamQuery.#plainTypes.includes(typeof item)) throw new TypeError('A stream parameter must be text, a number or a boolean.')
    return String(item)
  }

  /** @param {Record<string, unknown>} fields @returns {[string, unknown][]} */
  static #present(fields) {
    return Object.entries(fields).filter(([, item]) => item !== null)
  }

  /** @param {[string, unknown][]} entries @returns {string} */
  static #query(entries) {
    return new URLSearchParams(entries.map(([key, item]) => [key, TacStreamQuery.#plain(item)])).toString()
  }

  /** @param {string} path @param {string} query @returns {string} */
  static #join(path, query) {
    return query ? `${path}?${query}` : path
  }
}
