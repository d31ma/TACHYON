export default class PriceUtils {
  /**
   * @param {number} cents
   * @returns {string}
   */
  static label(cents) {
    return `$${PriceUtils.#dollars(cents)}`
  }

  /**
   * @param {number} cents
   * @returns {string}
   */
  static #dollars(cents) {
    return (cents / 100).toFixed(2)
  }
}
