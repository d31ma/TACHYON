import 'models/ProductReport.dart';

@RequestRoute("/products")
@Public
class ProductController {
  @Needs
  final ProductStore products;

  @GetRoute
  Future<ProductReport> getProduct() async {
    final activeStatus = 'active';
    final active = await products.findByStatus(activeStatus);
    final activeCount = await products.countByStatus(activeStatus);
    final others = await products.excluding(activeStatus, {'size': 5});
    return ProductReport(active, activeCount, others);
  }
}
