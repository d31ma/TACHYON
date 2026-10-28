[Setup]
class CatalogSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        adapters.http["catalog"] = new CatalogEchoDelegate();
        return adapters;
    }
}
