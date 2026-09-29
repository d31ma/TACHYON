@Helper
class TaxHelper {
  @Needs
  val rates: TaxRateHelper

  fun addTax(subtotal: Int): Int {
    val tax = rates.rate()
    return subtotal + tax
  }
}
