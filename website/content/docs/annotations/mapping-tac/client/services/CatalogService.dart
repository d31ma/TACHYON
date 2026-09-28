@Service
class CatalogService {
  String title = 'Not mapped.';

  void load() {
    final context = Behaviors.create(CatalogSetup, partition: 'public-catalog', signal: BehaviorSignal());
    final mapper = ProductConverter(context);
    final product = {
      'detail': {'id': 7},
      'name': 'Anvil',
      'extra': 'dropped',
    };
    final summary = mapper.toSummary(product) as Map;
    title = '${summary['kind']} ${summary['productId']}: ${summary['title']}';
    context.dispose();
  }
}
