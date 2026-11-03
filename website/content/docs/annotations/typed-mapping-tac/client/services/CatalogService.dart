import 'models/Detail.dart';
import 'models/Product.dart';
import 'models/Summary.dart';
@Service
class CatalogService {
  @Needs
  final ProductConverter productConverter;
  String result = '';

  void load() {
    const detail = Detail('detail', 'detail-private');
    final product = productConverter.copy(Product(7, 'Anvil', detail, [detail], 'product-private'));
    result = _text(productConverter.map(product));
  }

  String _text(Summary summary) {
    return '${summary.kind} ${summary.id}: ${summary.name}, ${summary.detail.name}, ${summary.items.length}';
  }
}
