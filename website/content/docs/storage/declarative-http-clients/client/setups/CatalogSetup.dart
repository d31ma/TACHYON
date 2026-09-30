@Setup
class CatalogSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters(
      http: {
        'catalog': BehaviorHttpAdapter(exchange: (request, execution) async => ['anvil', 'rope']),
      },
    );
  }
}
