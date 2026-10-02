[HTTPClient({"path":"/api/cached-probe","adapter":"cachedProbe","setup":"CachedHttpSetup"})]
[CacheSettings({"name":"dotnetOfflineProbe","ttlMs":60000,"maxBytes":65536,"maxEntries":16})]
public interface OfflineProbeClient
{
    [Cached]
    [GetRequest]
    System.Threading.Tasks.Task<BehaviorJSON> read();
}
