@Delegate
@Lifetime("transient")
class LedgerDelegate {
  @Needs
  val amounts: LedgerAmountsDelegate

  fun settle(subtotal: Int): Int {
    val surcharge = amounts.surcharge()
    return subtotal + surcharge
  }
}
