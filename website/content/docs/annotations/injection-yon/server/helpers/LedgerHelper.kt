@Helper
@Lifetime("transient")
class LedgerHelper {
  @Needs
  val amounts: LedgerAmountsHelper

  fun settle(subtotal: Int): Int {
    val surcharge = amounts.surcharge()
    return subtotal + surcharge
  }
}
