[Setup]
public sealed class CatalogSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        adapters.entities["catalog"] = new CatalogAdapterDelegate();
        adapters.authorizeEntity = (command, current) =>
            System.Threading.Tasks.Task.FromResult(command.Get("entity").Text == "ProductEntity");
        return adapters;
    }
}
