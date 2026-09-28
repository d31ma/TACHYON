@Helper
class CartHelper {
  @Needs
  final CartStore cartStore;

  int settle(int subtotal) {
    final surcharge = cartStore.surcharge();
    return subtotal + surcharge;
  }
}
