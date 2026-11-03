import models.CatalogSummary

@RequestRoute("/catalog")
@Public
class CatalogController {
    @Needs
    val converter: ProductConverter

    @GetRoute
    fun getCatalog(): CatalogSummary {
        val summary = converter.toSummary(
            mapOf("detail" to mapOf("id" to 7), "name" to "Anvil", "extra" to "dropped"))
        return CatalogSummary(summary)
    }
}
