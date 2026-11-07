@RequestRoute("/catalog")
@Public
class CatalogController {
    @Needs
    val catalog: CatalogClient

    @GetRoute
    fun getCatalog(): Any? = mapOf(
        "found" to catalog.find("anvil", "full", "acme"),
        "saved" to catalog.save(mapOf("sku" to "anvil", "level" to 3)),
        "replaced" to catalog.replace(5L, mapOf("sku" to "anvil", "level" to 4)),
        "renamed" to catalog.rename(6L, mapOf("sku" to "hammer", "level" to 1)),
        "removed" to catalog.remove(7L),
    )
}
