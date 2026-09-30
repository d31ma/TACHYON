@Service
export default class CheckoutService {
  /** @type {TaxHelper} */
  @Needs
  taxHelper;

  /** @returns {number} */
  total() {
    const subtotal = this._subtotal();
    return this.taxHelper.addTax(subtotal);
  }

  /** @returns {number} */
  _subtotal() {
    return 40;
  }
}
