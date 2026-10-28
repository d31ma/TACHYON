@Delegate
export default class BillingDelegate {
  static _SURCHARGE = 2;

  /**
   * @param {number} subtotal
   * @returns {number}
   */
  settle(subtotal) {
    return subtotal + BillingDelegate._SURCHARGE;
  }
}
