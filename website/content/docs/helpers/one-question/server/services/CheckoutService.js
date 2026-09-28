@Service
class CheckoutService {
  /** @type {TaxHelper} */
  @Needs
  taxHelper;

  /** @returns {number} */
  total() {
    const subtotal = this.#subtotal();
    return this.taxHelper.addTax(subtotal);
  }

  /** @returns {number} */
  #subtotal() {
    return 40;
  }
}
