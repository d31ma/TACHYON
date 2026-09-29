@Helper
export default class TaxHelper {
  /** @type {TaxRateStore} */
  @Needs
  taxRateStore;

  /**
   * @param {number} subtotal
   * @returns {number}
   */
  addTax(subtotal) {
    const tax = this.taxRateStore.rate();
    return subtotal + tax;
  }
}
