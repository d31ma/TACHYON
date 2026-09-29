@Service
@Lifetime({"value":"prototype"})
export default class CartService {
  /** @type {CartPricesHelper} */
  @Needs
  cartPricesHelper;
  /** @type {CartHelper} */
  @Needs
  cartHelper;

  /** @returns {number} */
  get total() {
    const subtotal = this.cartPricesHelper.subtotal();
    return this.cartHelper.settle(subtotal);
  }
}
