@RequestRoute("/catalog")
@Public
class CatalogController {
    @Needs
    val catalog: CatalogService
    @GetRoute
    fun getCatalog(): Any? = catalog.products()
}
