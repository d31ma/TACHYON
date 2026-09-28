import models.CatalogSummary

@RequestRoute("/catalog")
@Public
class CatalogController {
    @GetRoute
    fun getCatalog(): CatalogSummary {
        val context = Behaviors.create("CatalogSetup", Behaviors.Options("public-catalog", Behaviors.Signal()))
        val mapper = ProductConverter(context)
        val summary = mapper.toSummary(
            mapOf("detail" to mapOf("id" to 7), "name" to "Anvil", "extra" to "dropped"))
        context.close()
        return CatalogSummary(summary)
    }
}
