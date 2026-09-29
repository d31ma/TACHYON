@Helper
class TaxHelper {
  @Needs
  final TaxRateHelper rates;

  int addTax(int subtotal) {
    final tax = rates.rate();
    return subtotal + tax;
  }
}
