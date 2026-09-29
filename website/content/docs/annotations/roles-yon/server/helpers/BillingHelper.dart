@Helper
class BillingHelper {
  static const int _surcharge = 2;

  int settle(int subtotal) {
    return subtotal + _surcharge;
  }
}
