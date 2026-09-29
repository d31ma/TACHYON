[Helper]
class BillingHelper {
  const int Surcharge = 2;

  public int settle(int subtotal) {
    return subtotal + Surcharge;
  }
}
