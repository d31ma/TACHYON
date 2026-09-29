import models.LedgerReport

@RequestRoute("/ledger")
@Public
class LedgerController {
    @Needs
    val ledger: LedgerService

    @GetRoute
    fun getLedger(): LedgerReport {
        val settled = ledger.settle("open")
        return LedgerReport(settled, LedgerAdapterHelper.commands)
    }
}
