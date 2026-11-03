@Delegate
class TaxDelegate {
  @Needs
  val rates: TaxRateDelegate

  fun addTax(subtotal: Int): Int {
    val tax = rates.rate()
    return subtotal + tax
  }
}
