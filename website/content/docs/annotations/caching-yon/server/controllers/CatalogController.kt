import models.CacheReport

@RequestRoute("/catalog")
@Public
class CatalogController {
    @Needs
    val catalog: CatalogService

    @GetRoute
    fun getCatalog(): CacheReport {
        val sku = "anvil"
        val first = catalog.find(sku)
        val repeat = catalog.find(sku)
        catalog.write(sku, 99)
        val written = catalog.find(sku)
        catalog.invalidate(sku)
        val refreshed = catalog.find(sku)
        return CacheReport(first, repeat, written, refreshed)
    }
}
