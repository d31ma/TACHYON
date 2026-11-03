import 'models/Product.dart';
import 'models/Summary.dart';
@Converter
abstract interface class ProductConverter {
  @Convert({"fields":[{"target":"kind","constant":"product"}]})
  Summary map(Product product);

  Product copy(Product product);
}
