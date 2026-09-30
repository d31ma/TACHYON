import models.CatalogSummary

@RequestRoute("/catalog")
@Public
class CatalogController {
    @Needs
    ProductConverter converter

    @GetRoute
    CatalogSummary getCatalog() {
        Object summary = converter.toSummary(java.util.Map.of(
            "detail", java.util.Map.of("id", 7), "name", "Anvil", "extra", "dropped"))
        return new CatalogSummary(summary: summary)
    }
}
