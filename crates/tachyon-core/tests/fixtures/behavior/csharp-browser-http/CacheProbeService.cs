[Service]
[CacheSettings({"name":"probe","ttlMs":60000,"maxBytes":65536,"maxEntries":16})]
public sealed class CacheProbeService
{
    [Needs]
    private readonly BehaviorContext behaviors;

    [Cached]
    [WebCall({"client":"probe","method":"GET","path":"/api/probe"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> Read()
    {
        await System.Threading.Tasks.Task.CompletedTask;
        return BehaviorJSON.Parse("{}");
    }

    [ClearCache({"method":"Read"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> Clear()
    {
        await System.Threading.Tasks.Task.CompletedTask;
        return BehaviorJSON.From(null);
    }
}
