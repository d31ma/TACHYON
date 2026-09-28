@Helper
class BillingHelper {
  @Needs
  final BillingClient client;

  int settle(int subtotal) {
    final surcharge = client.surcharge();
    return subtotal + surcharge;
  }
}
