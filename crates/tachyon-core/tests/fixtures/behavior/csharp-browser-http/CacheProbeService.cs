[Service]
[CacheSettings({"name":"probe","ttlMs":60000,"maxBytes":65536,"maxEntries":16})]
public sealed class CacheProbeService
{
    [Needs]
    private readonly BehaviorContext behaviors;

    [Cached]
    [WebCall({"client":"probe","method":"GET","path":"/api/probe"})]
    public System.Threading.Tasks.Task<BehaviorJSON> Read() => System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{}"));

    [ClearCache({"method":"Read"})]
    public System.Threading.Tasks.Task<BehaviorJSON> Clear() => System.Threading.Tasks.Task.FromResult(BehaviorJSON.From(null));
}
