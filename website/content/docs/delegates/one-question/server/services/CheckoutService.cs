[Service]
class CheckoutService {
  [Needs]
  TaxDelegate _helper;

  public int total() {
    int amount = subtotal();
    return _helper.addTax(amount);
  }

  private int subtotal() {
    return 40;
  }
}
