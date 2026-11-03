import models.LedgerReport;

@RequestRoute("/ledger")
@Public
class LedgerController {
    private static final String OPEN = "open";

    @Needs
    LedgerService ledger;

    @GetRoute
    LedgerReport getLedger() {
        Object settled = ledger.settle(OPEN);
        refuse();
        return new LedgerReport(settled, LedgerAdapterDelegate.commands);
    }

    private void refuse() {
        try {
            ledger.refuse(OPEN);
        } catch (IllegalStateException refused) {
            LedgerAdapterDelegate.commands.add(refused.getMessage());
        }
    }
}
