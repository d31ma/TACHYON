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

    [AllOrNothing({"adapter":"ledger"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> refuse(BehaviorJSON status) {
        await Entries.save(BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> {
            ["id"] = BehaviorJSON.From(2),
            ["status"] = status,
        }));
        throw new System.InvalidOperationException("the ledger refused the entry");
    }
}
