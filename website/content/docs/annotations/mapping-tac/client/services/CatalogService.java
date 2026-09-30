@Service
class CatalogService {
    @Needs
    ProductConverter productConverter;
    String title = "Not mapped.";

    void load() {
        Object summary = productConverter.toSummary(java.util.Map.of(
            "detail", java.util.Map.of("id", 7), "name", "Anvil", "extra", "dropped"));
        title = titleOf((java.util.Map<?, ?>) summary);
    }

    private String titleOf(java.util.Map<?, ?> summary) {
        return summary.get("kind") + " " + summary.get("productId") + ": " + summary.get("title");
    }
}
