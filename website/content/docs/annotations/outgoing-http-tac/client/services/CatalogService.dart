@Service
class CatalogService {
  @Needs
  final CatalogClient catalog;
  int count = 0;

  Future<void> load() async {
    final answer = await catalog.all() as Map;
    count = (answer['items'] as List).length;
  }
}
