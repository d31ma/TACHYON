[CacheSettings({"name":"products","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
[Service]
public sealed class AnnotatedCatalogService {
    public BehaviorContext behaviors;
    private int calls = 0;
    [Cached]
    public async System.Threading.Tasks.Task<BehaviorJSON> list([PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})] BehaviorJSON page) {
        await System.Threading.Tasks.Task.CompletedTask;
        calls++;
        var offset = "offset";
        return BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["calls"] = BehaviorJSON.From(calls), [offset] = page.Get(offset) });
    }
    public BehaviorJSON page([PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})] BehaviorJSON input) { return input; }
    [UpdateCache({"method":"list"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> update(BehaviorJSON value, BehaviorJSON page) {
        await System.Threading.Tasks.Task.CompletedTask;
        return value;
    }
    [ClearCache({"method":"list"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> remove(BehaviorJSON page) {
        await System.Threading.Tasks.Task.CompletedTask;
        return BehaviorJSON.From(null);
    }
}

