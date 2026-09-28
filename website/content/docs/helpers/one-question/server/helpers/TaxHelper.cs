[Helper]
class TaxHelper {
  [Needs]
  TaxRateStore _store;

  public int addTax(int subtotal) {
    int tax = _store.rate();
    return subtotal + tax;
  }
}
