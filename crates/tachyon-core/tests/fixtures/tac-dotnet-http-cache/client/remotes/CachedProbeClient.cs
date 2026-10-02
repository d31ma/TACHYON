[HTTPClient({"path":"/api/cached-probe","adapter":"cachedProbe","setup":"CachedHttpSetup"})]
[CacheSettings({"name":"dotnetPublicProbe","ttlMs":60000,"maxBytes":65536,"maxEntries":16})]
public sealed class CachedProbeClient
{
    [Cached]
    [GetRequest]
    public async System.Threading.Tasks.Task<BehaviorJSON> read() {}

    [ClearCache({"method":"read"})]
    [PostRequest]
    public async System.Threading.Tasks.Task<BehaviorJSON> save() {}
}
