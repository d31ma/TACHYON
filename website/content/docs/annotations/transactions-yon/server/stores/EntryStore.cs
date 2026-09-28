[Store(typeof(EntryEntity), {"fields":["id","status"],"defaultSize":20,"maxSize":100})]
public sealed class EntryStore {
    [Needs]
    public BehaviorContext behaviors;
    public async System.Threading.Tasks.Task<BehaviorJSON> findByStatus(BehaviorJSON status) {}
    public async System.Threading.Tasks.Task<BehaviorJSON> countByStatus(BehaviorJSON status) {}

    [AllOrNothing({"adapter":"ledger"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> settle(BehaviorJSON status) {
        var open = await findByStatus(status);
        var total = await countByStatus(status);
        return BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> {
            ["open"] = open,
            ["total"] = total,
        });
    }
}
