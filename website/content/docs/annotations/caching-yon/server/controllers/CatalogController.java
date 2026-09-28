import models.CacheReport;

@RequestRoute("/catalog")
@Public
class CatalogController {
    @Needs
    final CatalogService catalog;

    @GetRoute
    CacheReport getCatalog() {
        String sku = "anvil";
        Object first = catalog.find(sku);
        Object repeat = catalog.find(sku);
        catalog.write(sku, 99);
        Object written = catalog.find(sku);
        catalog.invalidate(sku);
        Object refreshed = catalog.find(sku);
        return new CacheReport(first, repeat, written, refreshed);
    }
}
