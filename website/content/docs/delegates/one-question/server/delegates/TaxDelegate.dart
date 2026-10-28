@Delegate
class TaxDelegate {
  @Needs
  final TaxRateDelegate rates;

  int addTax(int subtotal) {
    final tax = rates.rate();
    return subtotal + tax;
  }
}
