@Helper
class TaxHelper {
  @Needs
  TaxRateHelper rates

  int addTax(int subtotal) {
    int tax = rates.rate()
    return subtotal + tax
  }
}
