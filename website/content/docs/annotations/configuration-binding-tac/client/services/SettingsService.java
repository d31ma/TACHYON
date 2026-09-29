@Service
class SettingsService {
    String title = "Catalog";
    long pageSize = 20;

    @OnStart
    void bind() {
        java.util.Map<String, Object> publicCatalog = java.util.Map.of(
            "title", "Catalog",
            "page", java.util.Map.of("size", 20),
            "cache", java.util.Map.of("enabled", true));
        java.util.Map<String, Object> published = java.util.Map.of("tac", java.util.Map.of("catalog", publicCatalog));
        String setup = "CatalogSetup";
        java.util.Map<String, Object> catalog = Behaviors.properties(setup, published);
        java.util.Map<String, Object> values = Behaviors.values("PublicValues", published);
        title = (String) values.get("title");
        pageSize = ((Number) catalog.get("pageSize")).longValue();
        Behaviors.Options options = new Behaviors.Options(
            "public-catalog", new Behaviors.Signal(), 10000, java.util.List.of("public"), published);
        Behaviors.create(setup, options).close();
    }
}
