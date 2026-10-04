@Helper
export default class CartHelper {
  @Needs
  cartPricesHelper!: CartPricesHelper;

  settle(subtotal: number): number {
    const surcharge = this.cartPricesHelper.surcharge();
    return subtotal + surcharge;
  }

  count(): number {
    return this.cartPricesHelper.count();
  }
}
