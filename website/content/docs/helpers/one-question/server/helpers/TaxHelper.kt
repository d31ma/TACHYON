@Helper
class TaxHelper {
  @Needs
  val store: TaxRateStore

  fun addTax(subtotal: Int): Int {
    val tax = store.rate()
    return subtotal + tax
  }
}
