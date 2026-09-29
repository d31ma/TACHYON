[Helper]
class TaxHelper {
  [Needs]
  TaxRateHelper _rates;

  public int addTax(int subtotal) {
    int tax = _rates.rate();
    return subtotal + tax;
  }
}
