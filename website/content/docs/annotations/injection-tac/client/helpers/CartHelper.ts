@Helper
class CartHelper {
  @Needs
  cartStore!: CartStore;

  settle(subtotal: number): number {
    const surcharge = this.cartStore.surcharge();
    return subtotal + surcharge;
  }
}
