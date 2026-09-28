@Helper
class BillingHelper {
  @Needs
  val client: BillingClient

  fun settle(subtotal: Int): Int {
    val surcharge = client.surcharge()
    return subtotal + surcharge
  }
}
