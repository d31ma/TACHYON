@Helper
class TaxHelper {
  @Needs
  TaxRateStore store;

  int addTax(int subtotal) {
    int tax = store.rate();
    return subtotal + tax;
  }
}
