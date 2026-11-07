import models.Ledger

@RequestRoute("/ledger")
@Public
class LedgerController {
  @Needs
  @Named("LedgerService")
  LedgerService service
  @Needs
  @Named("LedgerDelegate")
  LedgerDelegate settlement

  @GetRoute
  Ledger getLedger() {
    int subtotal = service.subtotal()
    int total = settlement.settle(subtotal)
    return new Ledger(total)
  }
}
