/** @import { Ledger } from '/models/Ledger.ts' */
@RequestRoute("/ledger")
@Public
class LedgerController {
  /** @type {LedgerService} */
  @Needs
  @Named("LedgerService")
  service;
  /** @type {LedgerHelper} */
  @Needs
  @Named("LedgerHelper")
  settlement;

  /** @returns {Ledger} */
  @GetRoute
  getLedger() {
    const subtotal = this.service.subtotal();
    const total = this.settlement.settle(subtotal);
    return { total };
  }
}
