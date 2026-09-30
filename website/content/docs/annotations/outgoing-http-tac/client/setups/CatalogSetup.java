@Setup
class CatalogSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration(
            java.util.Map.of(),
            java.util.Map.of("catalog", (request, current) -> java.util.Map.of("items", java.util.List.of("anvil"))),
            java.util.Map.of(), null);
    }
}
