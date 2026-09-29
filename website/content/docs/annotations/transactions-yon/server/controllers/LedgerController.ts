import type { LedgerReport } from '/models/LedgerReport.ts'

@RequestRoute("/ledger")
@Public
class LedgerController {
  @Needs
  readonly ledgerService!: LedgerService;

  @GetRoute
  async getLedger(): Promise<LedgerReport> {
    const settled = await this.ledgerService.settle('open');
    return { settled, commands: LedgerSetup.commands };
  }
}
