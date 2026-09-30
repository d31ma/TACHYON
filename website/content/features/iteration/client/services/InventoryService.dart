import 'models/Product.dart';

@Service
class InventoryService {
  List<Product> products = [const Product('Anvil'), const Product('Rope')];
}
