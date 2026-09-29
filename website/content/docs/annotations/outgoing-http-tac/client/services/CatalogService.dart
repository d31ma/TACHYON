@Service
class CatalogService {
  final CatalogClient catalog = CatalogClient();
  int count = 0;

  Future<void> load() async {
    final answer = await catalog.all() as Map;
    count = (answer['items'] as List).length;
  }
}
