import 'models/Product.dart';

@Service
class StockService {
  List<Product> products = [];
  bool restocking = true;
}
