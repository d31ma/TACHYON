@Service
@Lifetime({"value":"prototype"})
class CartService {
  @Needs
  final CartPricesHelper cartPricesHelper;
  @Needs
  final CartHelper cartHelper;

  int total() {
    final subtotal = cartPricesHelper.subtotal();
    return cartHelper.settle(subtotal);
  }
}
