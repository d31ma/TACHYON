@Setup
class CatalogSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration(java.util.Map.of("catalog", new MemoryCacheDelegate()),
            java.util.Map.of(), java.util.Map.of(), (call, current) -> true)
    }
}
