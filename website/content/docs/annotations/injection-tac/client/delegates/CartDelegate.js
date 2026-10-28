@Delegate
export default class CartDelegate {
  /** @type {CartPricesDelegate} */
  @Needs
  cartPricesDelegate;

  /**
   * @param {number} subtotal
   * @returns {number}
   */
  settle(subtotal) {
    const surcharge = this.cartPricesDelegate.surcharge();
    return subtotal + surcharge;
  }

  /** @returns {number} */
  count() {
    return this.cartPricesDelegate.count();
  }
}
