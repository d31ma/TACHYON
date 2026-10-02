[HTTPClient({"path":"/api/cached-probe","adapter":"cachedProbe","setup":"CachedHttpSetup"})]
[CacheSettings({"name":"dotnetWorkerProbe","ttlMs":60000,"maxBytes":65536,"maxEntries":16})]
public interface WorkerProbeClient
{
    [Cached]
    [GetRequest]
    System.Threading.Tasks.Task<BehaviorJSON> read();
}
