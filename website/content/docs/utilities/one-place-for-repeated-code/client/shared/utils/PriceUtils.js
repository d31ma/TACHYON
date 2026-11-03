export default class PriceUtils {
  /**
   * @param {number} cents
   * @returns {string}
   */
  static label(cents) {
    return `$${PriceUtils._dollars(cents)}`
  }

  /**
   * @param {number} cents
   * @returns {string}
   */
  static _dollars(cents) {
    return (cents / 100).toFixed(2)
  }
}
