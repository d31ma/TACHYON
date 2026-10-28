[Setup]
public sealed class LedgerSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        adapters.entities["ledger"] = new LedgerAdapterDelegate();
        adapters.authorizeEntity = (command, current) => System.Threading.Tasks.Task.FromResult(true);
        return adapters;
    }
}
