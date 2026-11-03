import 'models/Product.dart';
import 'models/Detail.dart';
import 'models/Summary.dart';
@RequestRoute("/catalog")
@Public
class CatalogController {
  @Needs
  final ProductConverter converter;
  @GetRoute
  Summary getCatalog() {
    const detail = Detail('detail', 'detail-private');
    return converter.map(Product(7, 'Anvil', detail, [detail], 'product-private'));
  }
}
