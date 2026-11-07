[Setup]
public class CatalogSetup {
    sealed class CatalogTransport : BehaviorHTTPAdapter {
        public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution) {
            return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"items\":[\"anvil\"]}"));
        }
    }

    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        adapters.http["catalog"] = new CatalogTransport();
        return adapters;
    }
}
