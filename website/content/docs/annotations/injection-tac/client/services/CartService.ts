@Service
@Lifetime({"value":"prototype"})
export default class CartService {
  @Needs
  cartPricesDelegate!: CartPricesDelegate;
  @Needs
  cartDelegate!: CartDelegate;

  total(): number {
    const subtotal = this.cartPricesDelegate.subtotal();
    return this.cartDelegate.settle(subtotal);
  }
}
