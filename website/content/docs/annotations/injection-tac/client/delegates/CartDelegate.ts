@Delegate
export default class CartDelegate {
  @Needs
  cartPricesDelegate!: CartPricesDelegate;

  settle(subtotal: number): number {
    const surcharge = this.cartPricesDelegate.surcharge();
    return subtotal + surcharge;
  }

  count(): number {
    return this.cartPricesDelegate.count();
  }
}
