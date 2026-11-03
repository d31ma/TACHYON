@Service
@CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
class CatalogService {
  late BehaviorContext behaviors;
  String sku = 'anvil';
  int reads = 0;

  @OnStart
  void open() {
    behaviors = Behaviors.create(CatalogSetup, partition: 'public-catalog', signal: BehaviorSignal());
  }

  @Cached
  Future<Object?> find(String sku) async {
    reads += 1;
    return _entry(sku, reads);
  }

  @UpdateCache({"method":"find"})
  Future<Object?> write(String sku, int reads) async {
    return _entry(sku, reads);
  }

  @ClearCache({"method":"find"})
  Future<Object?> invalidate(String sku) async {
    return _entry(sku, reads);
  }

  Map<String, Object?> _entry(String sku, int reads) => {'sku': sku, 'reads': reads};
}
