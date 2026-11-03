[RequestRoute("/catalog")]
[Public]
class CatalogController {
    [Needs]
    readonly CatalogService _catalog;
    [GetRoute]
    public object GetCatalog() { return _catalog.Products(); }
}
