import models.Ledger

@RequestRoute("/ledger")
@Public
class LedgerController {
  @Needs
  @Named("LedgerService")
  LedgerService service
  @Needs
  @Named("LedgerHelper")
  LedgerHelper settlement

  @GetRoute
  Ledger getLedger() {
    int subtotal = service.subtotal()
    int total = settlement.settle(subtotal)
    return new Ledger(total)
  }
}
