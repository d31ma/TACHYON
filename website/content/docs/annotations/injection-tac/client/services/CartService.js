@Service
@Lifetime({"value":"prototype"})
export default class CartService {
  /** @type {CartStore} */
  @Needs
  cartStore;
  /** @type {CartHelper} */
  @Needs
  cartHelper;

  /** @returns {number} */
  get total() {
    const subtotal = this.cartStore.subtotal();
    return this.cartHelper.settle(subtotal);
  }
}
