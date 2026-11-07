[Delegate]
class BillingDelegate {
  private const int Surcharge = 2;

  public int settle(int subtotal) {
    return subtotal + Surcharge;
  }
}
