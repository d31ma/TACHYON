import type { Ledger } from '/models/Ledger.ts'

@RequestRoute("/ledger")
@Public
class LedgerController {
  @Needs
  @Named("LedgerService")
  service: LedgerService;
  @Needs
  @Named("LedgerHelper")
  settlement: LedgerHelper;

  @GetRoute
  getLedger(): Ledger {
    const subtotal = this.service.subtotal();
    const total = this.settlement.settle(subtotal);
    return { total };
  }
}
