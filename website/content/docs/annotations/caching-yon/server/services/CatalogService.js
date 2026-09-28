@Service
@CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
class CatalogService {
  behaviors;
  reads = 0;

  @OnStart
  async open() {
    const signal = new AbortController().signal;
    this.behaviors = await Behaviors.create(CatalogSetup, { partition: 'public-catalog', signal });
  }

  @Cached
  async find(sku) {
    this.reads += 1;
    return { sku, reads: this.reads };
  }

  @UpdateCache({"method":"find"})
  async write(sku, reads) { return { sku, reads }; }

  @ClearCache({"method":"find"})
  async invalidate(sku) { return { sku }; }
}
