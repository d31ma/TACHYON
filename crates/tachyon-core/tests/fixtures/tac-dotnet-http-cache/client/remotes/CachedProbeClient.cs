[HTTPClient({"path":"/api/cached-probe","adapter":"cachedProbe","setup":"CachedHttpSetup"})]
[CacheSettings({"name":"dotnetPublicProbe","ttlMs":60000,"maxBytes":65536,"maxEntries":16})]
public interface CachedProbeClient
{
    [Cached]
    [GetRequest]
    System.Threading.Tasks.Task<BehaviorJSON> read();

    [ClearCache({"method":"read"})]
    [PostRequest]
    System.Threading.Tasks.Task<BehaviorJSON> save();

    [GetRequest("/slow")]
    System.Threading.Tasks.Task<BehaviorJSON> slow();
}
