@Service
class CatalogService {
  @Needs
  final ProductConverter converter;
  String title = 'Not mapped.';

  void load() {
    final product = {
      'detail': {'id': 7},
      'name': 'Anvil',
      'extra': 'dropped',
    };
    final summary = converter.toSummary(product) as Map;
    title = '${summary['kind']} ${summary['productId']}: ${summary['title']}';
  }
}
