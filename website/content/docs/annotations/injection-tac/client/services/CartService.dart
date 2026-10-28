@Service
@Lifetime({"value":"prototype"})
class CartService {
  @Needs
  final CartPricesDelegate cartPricesDelegate;
  @Needs
  final CartDelegate cartDelegate;

  int total() {
    final subtotal = cartPricesDelegate.subtotal();
    return cartDelegate.settle(subtotal);
  }
}
