[Service]
class BillingService {
  private const int Subtotal = 40;

  [Needs]
  BillingHelper _helper;

  public int total() {
    return _helper.settle(Subtotal);
  }
}
