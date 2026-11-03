@Service
class ProductService {
  @Needs
  final ProductStore products;
  String summary = 'Not loaded.';

  Future<void> load() async {
    final rows = await products.excluding('retired', {'size': 5}) as List;
    final activeCount = await products.countByStatus('active');
    summary = '${rows.length} of $activeCount active';
  }
}
