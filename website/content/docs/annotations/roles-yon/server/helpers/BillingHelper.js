@Helper
export default class BillingHelper {
  static SURCHARGE = 2;

  /**
   * @param {number} subtotal
   * @returns {number}
   */
  settle(subtotal) {
    return subtotal + BillingHelper.SURCHARGE;
  }
}
