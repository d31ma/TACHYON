import models.Ledger

@RequestRoute("/ledger")
@Public
class LedgerController {
  @Needs
  @Named("LedgerService")
  val service: LedgerService
  @Needs
  @Named("LedgerDelegate")
  val settlement: LedgerDelegate

  @GetRoute
  fun getLedger(): Ledger {
    val subtotal = service.subtotal()
    val total = settlement.settle(subtotal)
    return Ledger(total)
  }
}
