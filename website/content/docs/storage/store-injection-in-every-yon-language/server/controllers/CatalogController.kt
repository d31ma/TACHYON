@RequestRoute("/catalog")
@Public
class CatalogController {
  @Needs
  val catalog: CatalogStore
  @GetRoute
  fun getCatalog(): Any? = catalog.find()
}