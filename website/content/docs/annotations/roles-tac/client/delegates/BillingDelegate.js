@Delegate
export default class BillingDelegate {
  /**
   * @param {number} subtotal
   * @returns {number}
   */
  settle(subtotal) { return subtotal + 2; }
}
