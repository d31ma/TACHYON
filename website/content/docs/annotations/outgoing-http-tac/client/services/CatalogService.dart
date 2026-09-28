@Service
@HTTPClient
class CatalogService {
  late BehaviorContext behaviors;
  int count = 0;

  @OnStart
  void start() {
    behaviors = Behaviors.create(CatalogSetup, partition: 'catalog', signal: BehaviorSignal());
  }

  @GetRequest("/api/products")
  Future<Object?> all() async {}

  Future<void> load() async {
    final answer = await all() as Map;
    count = (answer['items'] as List).length;
  }
}
