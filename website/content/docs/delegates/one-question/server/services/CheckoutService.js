@Service
export default class CheckoutService {
  /** @type {TaxDelegate} */
  @Needs
  taxDelegate;

  /** @returns {number} */
  total() {
    const subtotal = this._subtotal();
    return this.taxDelegate.addTax(subtotal);
  }

  /** @returns {number} */
  _subtotal() {
    return 40;
  }
}
