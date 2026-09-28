@Helper
class TaxHelper {
  @Needs
  final TaxRateStore store;

  int addTax(int subtotal) {
    final tax = store.rate();
    return subtotal + tax;
  }
}
