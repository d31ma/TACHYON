import 'models/Product.dart';

@Service
class CatalogService {
  final List<Product> products = const [
    Product('anvil', 'Anvil', '£42.00'),
    Product('rope', 'Rope', '£8.00'),
  ];
}
