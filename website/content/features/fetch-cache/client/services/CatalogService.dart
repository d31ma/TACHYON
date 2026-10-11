@Service
class CatalogService {
  @Needs
  final ProductsClient productsClient;
  int count = 0;

  Future<void> load() async {
    final answer = await productsClient.list() as Map;
    count = (answer['items'] as List).length;
  }

  Future<void> add() async {
    await productsClient.save({'name': 'anvil'});
  }
}
