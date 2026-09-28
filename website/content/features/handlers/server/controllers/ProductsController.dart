import 'models/Product.dart';

@RequestRoute("/products")
@Public
class ProductsController {
  @GetRoute
  List<Product> getProducts() => [Product('anvil')];

  @PostRoute
  @ResponseStatus(201)
  Product postProduct(@FromBody Product product) => product;
}
