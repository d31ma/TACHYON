import models.LedgerReport;

@RequestRoute("/ledger")
@Public
class LedgerController {
    @GetRoute
    LedgerReport getLedger() {
        Behaviors.Options options = new Behaviors.Options("ledger", new Behaviors.Signal());
        Behaviors.Context context = Behaviors.create("LedgerSetup", options);
        EntryStore entries = new EntryStore(context);
        Object settled = entries.settle("open");
        context.close();
        return new LedgerReport(settled, LedgerAdapterHelper.commands);
    }
}
