class TacPreferenceBounds {
  static mostKeyBytes = 128
  static mostValueBytes = 4096

  static key(value) {
    TacPreferenceBounds.#string(value)
    TacPreferenceBounds.#nonempty(value)
    TacPreferenceBounds.#length(value, TacPreferenceBounds.mostKeyBytes)
    TacPreferenceBounds.#bytes(value, TacPreferenceBounds.mostKeyBytes)
  }

  static value(value) {
    TacPreferenceBounds.#string(value)
    TacPreferenceBounds.#length(value, TacPreferenceBounds.mostValueBytes)
    TacPreferenceBounds.#bytes(value, TacPreferenceBounds.mostValueBytes)
  }

  static #string(value) {
    if (typeof value !== 'string') throw new TypeError('PreferenceStrings needs raw strings.')
  }

  static #nonempty(value) {
    if (value.length === 0) throw new TypeError('PreferenceStrings needs a nonempty key.')
  }

  static #length(value, most) {
    if (value.length > most) throw new RangeError('PreferenceStrings exceeds its byte limit.')
  }

  static #bytes(value, most) {
    if (new TextEncoder().encode(value).byteLength > most) throw new RangeError('PreferenceStrings exceeds its byte limit.')
  }
}
