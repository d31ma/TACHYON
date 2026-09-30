@Setup
class ProductsSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters(
      http: {
        'products': BehaviorHttpAdapter(exchange: (request, execution) async => {
              'items': ['anvil', 'hammer'],
            }),
      },
    );
  }
}
