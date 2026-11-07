@Delegate
export default class CartPricesDelegate {
  /** @returns {number} */
  subtotal() { return 40; }
  /** @returns {number} */
  surcharge() { return 2; }
  /** @returns {number} */
  count() { return 3; }
}
