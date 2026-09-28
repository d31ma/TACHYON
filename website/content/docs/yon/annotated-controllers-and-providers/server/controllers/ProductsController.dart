import 'models/Product.dart';

@RequestRoute("/products")
@Public
class ProductsController {
  @Needs
  @Named("ProductsService")
  final ProductsService catalog;

  @GetRoute
  List<Product> getProducts() => catalog.list();

  @GetRoute("/{id}")
  Product getProduct(@FromPath int id) => catalog.find(id);
}
