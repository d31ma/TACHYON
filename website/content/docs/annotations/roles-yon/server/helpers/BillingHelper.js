@Helper
export default class BillingHelper {
  /** @type {BillingClient} */
  @Needs
  billingClient;

  /**
   * @param {number} subtotal
   * @returns {number}
   */
  settle(subtotal) {
    const surcharge = this.billingClient.surcharge();
    return subtotal + surcharge;
  }
}
