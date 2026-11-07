@Service
class LedgerService {
  @Needs
  val amounts: LedgerAmountsDelegate
  fun subtotal(): Int = amounts.subtotal()
}
