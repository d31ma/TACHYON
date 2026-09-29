[Service]
public sealed class LedgerService {
    public BehaviorContext behaviors;
    [Needs]
    public EntryStore Entries;

    [OnStart]
    public void open() {
        behaviors = Behaviors.create(typeof(LedgerSetup), "ledger", new BehaviorSignal());
    }

    [AllOrNothing({"adapter":"ledger"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> settle(BehaviorJSON status) {
        var open = await Entries.findByStatus(status);
        var total = await Entries.countByStatus(status);
        return BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> {
            ["open"] = open,
            ["total"] = total,
        });
    }
}
