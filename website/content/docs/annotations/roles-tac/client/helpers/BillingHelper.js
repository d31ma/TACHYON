@Helper
export default class BillingHelper {
  /**
   * @param {number} subtotal
   * @returns {number}
   */
  settle(subtotal) { return subtotal + 2; }
}
