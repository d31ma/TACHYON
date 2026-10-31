import models.LedgerReport

@RequestRoute("/ledger")
@Public
class LedgerController {
    companion object { private const val OPEN = "open" }

    @Needs
    val ledger: LedgerService

    @GetRoute
    fun getLedger(): LedgerReport {
        val settled = ledger.settle(OPEN)
        refuse()
        return LedgerReport(settled, LedgerAdapterDelegate.commands)
    }

    private fun refuse() {
        try {
            ledger.refuse(OPEN)
        } catch (refused: IllegalStateException) {
            LedgerAdapterDelegate.commands.add(refused.message.orEmpty())
        }
    }
}
