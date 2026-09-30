import models.Balance

@RequestRoute("/ledger")
@Public
class LedgerController {
    @Needs
    val ledgerClient: LedgerClient

    @GetRoute
    fun getLedger(): Balance = Balance(ledgerClient.balance("ACME-7"), ledgerClient.forecast(1L))
}
