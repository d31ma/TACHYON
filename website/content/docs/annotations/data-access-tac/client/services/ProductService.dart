@Service
class ProductService {
  String summary = 'Not loaded.';

  Future<void> load() async {
    final context = Behaviors.create(CatalogSetup, partition: 'public-catalog', signal: BehaviorSignal());
    final products = ProductStore(context);
    final page = products.bounded({'size': 5}) as Map<String, Object?>;
    final rows = await products.excluding('retired', page) as List;
    final activeCount = await products.countByStatus('active');
    summary = '${rows.length} of $activeCount active';
    context.dispose();
  }
}
