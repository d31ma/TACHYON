[Delegate]
class TaxDelegate {
  [Needs]
  TaxRateDelegate _rates;

  public int addTax(int subtotal) {
    int tax = _rates.rate();
    return subtotal + tax;
  }
}
