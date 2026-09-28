import models.Ledger

@RequestRoute("/ledger")
@Public
class LedgerController {
  @Needs
  @Named("LedgerService")
  val service: LedgerService
  @Needs
  @Named("LedgerHelper")
  val settlement: LedgerHelper

  @GetRoute
  fun getLedger(): Ledger {
    val subtotal = service.subtotal()
    val total = settlement.settle(subtotal)
    return Ledger(total)
  }
}
