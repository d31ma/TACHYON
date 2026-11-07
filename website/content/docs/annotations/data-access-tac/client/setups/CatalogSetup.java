@Setup
class CatalogSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        Behaviors.EntityAdapter catalog = (command, current) -> {
            java.util.Map<?, ?> query = (java.util.Map<?, ?>) command.get("query");
            return "count".equals(query.get("kind")) ? (Object) 0L : java.util.List.of();
        };
        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of("publicCatalog", catalog),
            null, (command, current) -> "ProductEntity".equals(command.get("entity")));
    }
}
