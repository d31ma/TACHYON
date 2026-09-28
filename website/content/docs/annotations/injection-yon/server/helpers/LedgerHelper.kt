@Helper
@Lifetime("transient")
class LedgerHelper {
  @Needs
  val repository: LedgerStore

  fun settle(subtotal: Int): Int {
    val surcharge = repository.surcharge()
    return subtotal + surcharge
  }
}
