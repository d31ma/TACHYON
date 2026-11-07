export default class PriceUtils {
  /**
   * @param {number} cents
   * @returns {number}
   */
  static withTax(cents) {
    PriceUtils._requirePositive(cents)
    return Math.round(cents * 1.2)
  }

  /**
   * @param {number} cents
   */
  static _requirePositive(cents) {
    if (cents < 0) throw new Error('A price is never negative.')
  }
}
