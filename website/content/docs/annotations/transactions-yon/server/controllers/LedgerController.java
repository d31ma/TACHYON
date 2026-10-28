import models.LedgerReport;

@RequestRoute("/ledger")
@Public
class LedgerController {
    @Needs
    LedgerService ledger;

    @GetRoute
    LedgerReport getLedger() {
        Object settled = ledger.settle("open");
        return new LedgerReport(settled, LedgerAdapterDelegate.commands);
    }
}
