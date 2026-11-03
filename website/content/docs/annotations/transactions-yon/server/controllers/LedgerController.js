/** @import { LedgerReport } from '/models/LedgerReport.ts' */
@RequestRoute("/ledger")
@Public
class LedgerController {
  /** @type {LedgerService} */
  @Needs
  ledgerService;

  /** @returns {Promise<LedgerReport>} */
  @GetRoute
  async getLedger() {
    const settled = await this.ledgerService.settle('open');
    return { settled, commands: LedgerSetup.commands };
  }
}
