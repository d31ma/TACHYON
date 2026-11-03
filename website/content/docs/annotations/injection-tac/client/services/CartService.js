@Service
@Lifetime({"value":"prototype"})
export default class CartService {
  /** @type {CartPricesDelegate} */
  @Needs
  cartPricesDelegate;
  /** @type {CartDelegate} */
  @Needs
  cartDelegate;

  /** @returns {number} */
  total() {
    const subtotal = this.cartPricesDelegate.subtotal();
    return this.cartDelegate.settle(subtotal);
  }
}
