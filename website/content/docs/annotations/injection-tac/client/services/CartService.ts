@Service
@Lifetime({"value":"prototype"})
export default class CartService {
  @Needs
  cartPricesHelper!: CartPricesHelper;
  @Needs
  cartHelper!: CartHelper;

  get total(): number {
    const subtotal = this.cartPricesHelper.subtotal();
    return this.cartHelper.settle(subtotal);
  }
}
