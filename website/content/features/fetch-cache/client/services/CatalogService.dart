@Service
class CatalogService {
  final ProductsClient _products = ProductsClient();
  int count = 0;

  Future<void> load() async {
    final answer = await _products.list() as Map;
    count = (answer['items'] as List).length;
  }

  Future<void> add() async {
    await _products.save({'name': 'anvil'});
  }
}
