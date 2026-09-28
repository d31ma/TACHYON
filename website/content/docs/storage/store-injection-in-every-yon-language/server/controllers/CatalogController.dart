@RequestRoute("/catalog")
@Public
class CatalogController {
  @Needs
  final CatalogStore catalog;

  @GetRoute
  Future<Object?> getCatalog() {
    return catalog.find();
  }
}
