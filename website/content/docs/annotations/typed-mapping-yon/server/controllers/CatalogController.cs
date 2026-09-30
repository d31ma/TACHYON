using Models;
[RequestRoute("/catalog")]
[Public]
class CatalogController {
[Needs]
ProductConverter _converter;
[GetRoute]
public Summary GetCatalog() {
var detail = new Detail("detail", "detail-private");
return _converter.map(new Product(7, "Anvil", detail, new[] {detail}, "product-private"));
}
}
