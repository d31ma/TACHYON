@Helper
class CartHelper {
  @Needs
  final CartPricesHelper cartPricesHelper;

  int settle(int subtotal) {
    final surcharge = cartPricesHelper.surcharge();
    return subtotal + surcharge;
  }
}
