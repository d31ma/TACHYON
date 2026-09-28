@RequestRoute("/catalog")
@Public
class CatalogController {
  @Needs
  CatalogStore catalog;
  @GetRoute
  Object getCatalog() { return catalog.find(); }
}