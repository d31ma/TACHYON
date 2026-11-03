[Service]
[CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
public sealed class CatalogService {
    public BehaviorContext behaviors;
    public int reads = 0;

    [OnStart]
    public void open() {
        behaviors = Behaviors.create(typeof(CatalogSetup), "public-catalog", new BehaviorSignal());
    }

    [Cached]
    public async System.Threading.Tasks.Task<BehaviorJSON> find(BehaviorJSON sku) {
        await System.Threading.Tasks.Task.CompletedTask;
        reads++;
        return Entry(sku, BehaviorJSON.From(reads));
    }

    [UpdateCache({"method":"find"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> write(BehaviorJSON sku, BehaviorJSON reads) {
        await System.Threading.Tasks.Task.CompletedTask;
        return Entry(sku, reads);
    }

    [ClearCache({"method":"find"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> invalidate(BehaviorJSON sku) {
        await System.Threading.Tasks.Task.CompletedTask;
        return Entry(sku, BehaviorJSON.From(reads));
    }

    private static BehaviorJSON Entry(BehaviorJSON sku, BehaviorJSON reads) {
        return BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> {
            ["sku"] = sku,
            ["reads"] = reads,
        });
    }
}
