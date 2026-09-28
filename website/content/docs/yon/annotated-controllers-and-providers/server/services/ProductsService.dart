import 'models/Product.dart';

@Service
@Lifetime("transient")
class ProductsService {
  List<Product> list() => [find(1)];

  Product find(int id) => Product(id, 'Anvil');
}
