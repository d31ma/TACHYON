import models.LedgerReport

@RequestRoute("/ledger")
@Public
class LedgerController {
    @GetRoute
    fun getLedger(): LedgerReport {
        val context = Behaviors.create("LedgerSetup", Behaviors.Options("ledger", Behaviors.Signal()))
        val entries = EntryStore(context)
        val settled = entries.settle("open")
        context.dispose()
        return LedgerReport(settled, LedgerAdapterHelper.commands)
    }
}
