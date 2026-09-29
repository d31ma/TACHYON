@RequestRoute("/catalog")
@Public
class CatalogController {
  @Needs
  CatalogService catalog
  @GetRoute
  Object getCatalog() { return catalog.find() }
}