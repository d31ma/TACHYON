class TacRandomId {
  /** @type {number} */
  static byteCount = 16
  /** @type {number} */
  static hexBase = 16
  /** @type {number} */
  static versionIndex = 6
  /** @type {number} */
  static variantIndex = 8
  /** @type {number[]} */
  static groupLengths = [8, 4, 4, 4, 12]

  /** @returns {string} */
  static next() {
    return TacRandomId.#made(globalThis.crypto)
  }

  /** @param {Crypto} source @returns {string} */
  static #made(source) {
    if (typeof source.randomUUID === 'function') return source.randomUUID()
    return TacRandomId.#built(source.getRandomValues(new Uint8Array(TacRandomId.byteCount)))
  }

  /** @param {Uint8Array} bytes @returns {string} */
  static #built(bytes) {
    bytes[TacRandomId.versionIndex] = (bytes[TacRandomId.versionIndex] & 0x0f) | 0x40
    bytes[TacRandomId.variantIndex] = (bytes[TacRandomId.variantIndex] & 0x3f) | 0x80
    return TacRandomId.#grouped(TacRandomId.#hex(bytes))
  }

  /** @param {Uint8Array} bytes @returns {string} */
  static #hex(bytes) {
    let text = ''
    for (const byte of bytes) text += byte.toString(TacRandomId.hexBase).padStart(2, '0')
    return text
  }

  /** @param {string} hex @returns {string} */
  static #grouped(hex) {
    const groups = []
    let start = 0
    for (const length of TacRandomId.groupLengths) {
      groups.push(hex.slice(start, start + length))
      start += length
    }
    return groups.join('-')
  }
}
