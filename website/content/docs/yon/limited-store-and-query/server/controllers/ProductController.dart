import 'models/ProductReport.dart';

@RequestRoute("/products")
@Public
class ProductController {
  @GetRoute
  Future<ProductReport> getProduct() async {
    final context = Behaviors.create(CatalogSetup, partition: 'public-catalog', signal: BehaviorSignal());
    final products = ProductStore(context);
    final activeStatus = 'active';
    final active = await products.findByStatus(activeStatus);
    final activeCount = await products.countByStatus(activeStatus);
    final page = products.bounded({'size': 5}) as Map<String, Object?>;
    final others = await products.excluding(activeStatus, page);
    context.dispose();
    return ProductReport(active, activeCount, others);
  }
}
