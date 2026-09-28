[Service]
class CheckoutService {
  [Needs]
  TaxHelper _helper;

  public int total() {
    int amount = subtotal();
    return _helper.addTax(amount);
  }

  private int subtotal() {
    return 40;
  }
}
