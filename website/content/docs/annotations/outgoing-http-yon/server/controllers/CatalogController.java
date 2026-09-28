@RequestRoute("/catalog")
@Public
class CatalogController {
    @Needs
    final CatalogService catalog;
    @GetRoute
    Object getCatalog() { return catalog.products(); }
}
