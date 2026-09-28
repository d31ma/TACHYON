@Service
@Lifetime({"value":"prototype"})
export default class CartService {
  @Needs
  cartStore!: CartStore;
  @Needs
  cartHelper!: CartHelper;

  get total(): number {
    const subtotal = this.cartStore.subtotal();
    return this.cartHelper.settle(subtotal);
  }
}
