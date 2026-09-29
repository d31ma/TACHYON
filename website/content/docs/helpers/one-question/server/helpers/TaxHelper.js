@Helper
export default class TaxHelper {
  /** @type {TaxRateHelper} */
  @Needs
  taxRateHelper;

  /**
   * @param {number} subtotal
   * @returns {number}
   */
  addTax(subtotal) {
    const tax = this.taxRateHelper.rate();
    return subtotal + tax;
  }
}
