[Helper]
class BillingHelper {
  [Needs]
  BillingClient _client;

  public int settle(int subtotal) {
    int surcharge = _client.surcharge();
    return subtotal + surcharge;
  }
}
