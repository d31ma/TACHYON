@RequestRoute("/catalog")
@Public
class CatalogController {
  @Needs
  final CatalogService catalog;

  @GetRoute
  Future<Object?> getCatalog() {
    return catalog.products();
  }
}
