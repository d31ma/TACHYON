@Delegate
class CartDelegate {
  @Needs
  final CartPricesDelegate cartPricesDelegate;

  int settle(int subtotal) {
    final surcharge = cartPricesDelegate.surcharge();
    return subtotal + surcharge;
  }

  int count() {
    return cartPricesDelegate.count();
  }
}
