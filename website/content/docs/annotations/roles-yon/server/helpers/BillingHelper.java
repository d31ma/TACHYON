@Helper
class BillingHelper {
  @Needs
  BillingClient client;

  int settle(int subtotal) {
    int surcharge = client.surcharge();
    return subtotal + surcharge;
  }
}
