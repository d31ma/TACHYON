[CacheSettings({"name":"products","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
[HTTPClient({"adapter": "catalog"})]
public sealed class AnnotatedValidatedCacheClient {
    public BehaviorContext behaviors;
    public int calls = 0;
    [Cached]
    public async System.Threading.Tasks.Task<BehaviorJSON> accept([Check(typeof(AnnotatedInput))] BehaviorJSON input) {
        await System.Threading.Tasks.Task.CompletedTask;
        calls++;
        return input;
    }
    public BehaviorJSON page([Check(typeof(AnnotatedPageInput))] [PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})] BehaviorJSON input) {
        calls++;
        return input;
    }
    [GetRequest("/products")]
    public async System.Threading.Tasks.Task<BehaviorJSON> remote([Check(typeof(AnnotatedInput))] BehaviorJSON input) {
        await System.Threading.Tasks.Task.CompletedTask;
        calls++;
        return BehaviorJSON.Parse("{}");
    }
}

