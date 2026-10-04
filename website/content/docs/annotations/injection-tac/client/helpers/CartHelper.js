@Helper
export default class CartHelper {
  /** @type {CartPricesHelper} */
  @Needs
  cartPricesHelper;

  /**
   * @param {number} subtotal
   * @returns {number}
   */
  settle(subtotal) {
    const surcharge = this.cartPricesHelper.surcharge();
    return subtotal + surcharge;
  }

  /** @returns {number} */
  count() {
    return this.cartPricesHelper.count();
  }
}
