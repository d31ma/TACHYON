import models.CatalogSummary

@RequestRoute("/catalog")
@Public
class CatalogController {
    @GetRoute
    CatalogSummary getCatalog() {
        Behaviors.Context context = Behaviors.create("CatalogSetup",
                new Behaviors.Options("public-catalog", new Behaviors.Signal()))
        ProductConverter mapper = new ProductConverter(context)
        Object summary = mapper.toSummary(java.util.Map.of(
            "detail", java.util.Map.of("id", 7), "name", "Anvil", "extra", "dropped"))
        context.close()
        return new CatalogSummary(summary: summary)
    }
}
