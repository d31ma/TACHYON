@Helper
export default class CartHelper {
  /** @type {CartStore} */
  @Needs
  cartStore;

  /**
   * @param {number} subtotal
   * @returns {number}
   */
  settle(subtotal) {
    const surcharge = this.cartStore.surcharge();
    return subtotal + surcharge;
  }
}
