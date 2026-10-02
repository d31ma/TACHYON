[HTTPClient({"path":"/api/cached-probe","adapter":"cached-probe","setup":"CachedHttpSetup"})]
[CacheSettings({"name":"dotnet-public-probe","ttlMs":60000,"maxBytes":65536,"maxEntries":16})]
public sealed class CachedProbeClient
{
    [Cached]
    [GetRequest]
    public async System.Threading.Tasks.Task<BehaviorJSON> read() {}

    [ClearCache({"method":"read"})]
    [PostRequest]
    public async System.Threading.Tasks.Task<BehaviorJSON> save() {}
}
