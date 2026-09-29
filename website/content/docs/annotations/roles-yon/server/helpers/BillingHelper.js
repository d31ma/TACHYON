@Helper
export default class BillingHelper {
  static _SURCHARGE = 2;

  /**
   * @param {number} subtotal
   * @returns {number}
   */
  settle(subtotal) {
    return subtotal + BillingHelper._SURCHARGE;
  }
}
