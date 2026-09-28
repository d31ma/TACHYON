/** @import { LedgerReport } from '/models/LedgerReport.ts' */
@RequestRoute("/ledger")
@Public
class LedgerController {
  /** @returns {Promise<LedgerReport>} */
  @GetRoute
  async getLedger() {
    const context = await Behaviors.create(LedgerSetup, {
      partition: 'ledger',
      signal: AbortSignal.timeout(5000),
    });
    const entries = Behaviors.construct(EntryStore, { behaviors: context });
    const settled = await entries.settle('open');
    context.dispose();
    return { settled, commands: LedgerSetup.commands };
  }
}
