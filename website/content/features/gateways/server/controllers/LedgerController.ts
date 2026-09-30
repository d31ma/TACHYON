import type { Balance } from '/models/Balance.ts'

@RequestRoute("/ledger")
@Public
class LedgerController {
  @Needs
  readonly ledgerClient!: LedgerClient;

  @GetRoute
  async getLedger(): Promise<Balance> {
    const balance = await this.ledgerClient.balance('ACME-7');
    const forecast = await this.ledgerClient.forecast(1);
    return { balance, forecast };
  }
}
