[Setup]
public class ProductsSetup {
    sealed class ProductsTransport : BehaviorHTTPAdapter {
        public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution) {
            return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"items\":[\"anvil\",\"hammer\"]}"));
        }
    }

    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        adapters.http["products"] = new ProductsTransport();
        return adapters;
    }
}
