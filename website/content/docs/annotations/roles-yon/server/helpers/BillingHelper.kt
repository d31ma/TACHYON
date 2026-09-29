@Helper
class BillingHelper {
  fun settle(subtotal: Int): Int {
    return subtotal + SURCHARGE
  }

  companion object {
    private const val SURCHARGE: Int = 2
  }
}
