import models.Balance;

@RequestRoute("/ledger")
@Public
class LedgerController {
    @Needs
    final LedgerClient ledgerClient;

    @GetRoute
    Balance getLedger() {
        return new Balance(ledgerClient.balance("ACME-7"), ledgerClient.forecast(1L));
    }
}
