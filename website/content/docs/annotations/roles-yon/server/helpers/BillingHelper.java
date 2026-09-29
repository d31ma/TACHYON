@Helper
class BillingHelper {
  private static final int SURCHARGE = 2;

  int settle(int subtotal) {
    return subtotal + SURCHARGE;
  }
}
