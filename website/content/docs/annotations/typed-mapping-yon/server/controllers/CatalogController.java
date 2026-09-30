import models.Product;
import models.Detail;
import models.Summary;
@RequestRoute("/catalog")
@Public
class CatalogController {
@Needs
ProductConverter converter;
@GetRoute
Summary getCatalog() {
Detail detail = new Detail("detail", "detail-private");
return converter.map(new Product(7, "Anvil", detail, java.util.List.of(detail), "product-private"));
}
}
