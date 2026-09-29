@Service
class ProductService {
  String summary = 'Not loaded.';

  Future<void> load() async {
    final products = ProductStore();
    final rows = await products.excluding('retired', {'size': 5}) as List;
    final activeCount = await products.countByStatus('active');
    summary = '${rows.length} of $activeCount active';
  }
}
