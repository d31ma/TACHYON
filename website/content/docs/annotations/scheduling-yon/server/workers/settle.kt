@Schedule({"fixedDelaySeconds":300})
class Handler {
  @Needs
  val ledger: LedgerService

  fun POST(): Int = ledger.total()
}
