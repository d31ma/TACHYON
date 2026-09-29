@Helper
class BillingHelper {
  static const int surcharge = 2;

  int settle(int subtotal) {
    return subtotal + surcharge;
  }
}
