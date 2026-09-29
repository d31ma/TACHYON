@Service
class LedgerService {
  @Needs
  val amounts: LedgerAmountsHelper
  fun subtotal(): Int = amounts.subtotal()
}
