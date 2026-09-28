@HTTPClient
class CatalogClient {
  late BehaviorContext behaviors;

  @GetRequest("/products")
  Future<Object?> all(@RequestParam("page") int page) async {}

  @PostRequest("/products")
  Future<Object?> add(@RequestBody Object? product) async {}

  @GetRequest("/products")
  Future<Object?> first() async {
    return {
      'query': {'page': 1},
    };
  }

  @OnStart
  void initialize() {
    behaviors = Behaviors.create(CatalogSetup, partition: 'catalog', signal: BehaviorSignal());
  }
}
