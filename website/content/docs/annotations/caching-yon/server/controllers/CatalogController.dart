import 'models/CacheReport.dart';

@RequestRoute("/catalog")
@Public
class CatalogController {
  @Needs
  final CatalogService catalog;

  @GetRoute
  Future<CacheReport> getCatalog() async {
    final sku = 'anvil';
    final first = await catalog.find(sku);
    final repeat = await catalog.find(sku);
    await catalog.write(sku, 99);
    final written = await catalog.find(sku);
    await catalog.invalidate(sku);
    final refreshed = await catalog.find(sku);
    return CacheReport(first, repeat, written, refreshed);
  }
}
