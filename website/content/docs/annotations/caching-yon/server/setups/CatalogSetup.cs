[Setup]
public sealed class CatalogSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        adapters.caches["catalog"] = new MemoryCacheDelegate();
        adapters.authorize = (call, current) => System.Threading.Tasks.Task.FromResult(true);
        return adapters;
    }
}
