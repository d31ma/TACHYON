import 'models/Product.dart';

@RequestRoute("/products")
@Public
class ProductsController {
  @Needs
  final ProductService productService;

  @PostRoute
  Product postProduct(@FromBody Product product) {
    productService.accept({'name': product.name, 'level': product.level});
    return product;
  }
}
