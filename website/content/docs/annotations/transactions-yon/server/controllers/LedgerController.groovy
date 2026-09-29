import models.LedgerReport

@RequestRoute("/ledger")
@Public
class LedgerController {
    @Needs
    final LedgerService ledger

    @GetRoute
    LedgerReport getLedger() {
        Object settled = ledger.settle("open")
        return new LedgerReport(settled, LedgerAdapterHelper.commands)
    }
}
