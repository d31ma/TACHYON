@Delegate
class TaxDelegate {
  @Needs
  TaxRateDelegate rates

  int addTax(int subtotal) {
    int tax = rates.rate()
    return subtotal + tax
  }
}
