[Setup]
public sealed class OrdersSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        adapters.authorize = (call, current) => System.Threading.Tasks.Task.FromResult(true);
        return adapters;
    }
}
