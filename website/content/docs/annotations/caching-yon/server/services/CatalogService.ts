@Service
@CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
class CatalogService {
  behaviors!: BehaviorContext;
  reads = 0;

  @OnStart
  async open() {
    const signal = new AbortController().signal;
    this.behaviors = await Behaviors.create(CatalogSetup, { partition: 'public-catalog', signal });
  }

  @Cached
  async find(sku: string) {
    this.reads += 1;
    return { sku, reads: this.reads };
  }

  @UpdateCache({"method":"find"})
  async write(sku: string, reads: number) { return { sku, reads }; }

  @ClearCache({"method":"find"})
  async invalidate(sku: string) { return { sku }; }
}
