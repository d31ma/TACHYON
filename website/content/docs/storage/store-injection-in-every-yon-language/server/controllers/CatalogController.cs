[RequestRoute("/catalog")]
[Public]
class CatalogController {
  [Needs]
  CatalogStore _catalog;
  [GetRoute]
  public object GetCatalog() { return _catalog.find(); }
}