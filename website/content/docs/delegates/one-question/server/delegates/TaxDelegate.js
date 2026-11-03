@Delegate
export default class TaxDelegate {
  /** @type {TaxRateDelegate} */
  @Needs
  taxRateDelegate;

  /**
   * @param {number} subtotal
   * @returns {number}
   */
  addTax(subtotal) {
    const tax = this.taxRateDelegate.rate();
    return subtotal + tax;
  }
}
