import 'models/Product.dart';

@Service
class CatalogService {
  String title = 'Catalog';
  int count = 6;
  Product product = const Product('Anvil', '£42.00');
}
