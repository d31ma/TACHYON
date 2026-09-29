[RequestRoute("/catalog")]
[Public]
class CatalogController {
  [Needs]
  CatalogService _catalog;
  [GetRoute]
  public object GetCatalog() { return _catalog.find(); }
}