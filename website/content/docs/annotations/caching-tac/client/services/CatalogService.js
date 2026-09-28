@Service
@CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
export default class CatalogService {
  /** @type {AbortSignal} */
  @Needs
  lifecycle;
  /** @type {BehaviorContext} */
  behaviors;
  sku = 'anvil';
  reads = 0;

  @OnStart
  async open() {
    this.behaviors = await Behaviors.create(CatalogSetup, { partition: 'public-catalog', signal: this.lifecycle });
  }

  /**
   * @param {string} sku
   * @returns {Promise<{ sku: string, reads: number }>}
   */
  @Cached
  async find(sku) {
    this.reads += 1;
    return { sku, reads: this.reads };
  }

  /**
   * @param {string} sku
   * @param {number} reads
   * @returns {Promise<{ sku: string, reads: number }>}
   */
  @UpdateCache({"method":"find"})
  async write(sku, reads) { return { sku, reads }; }

  /**
   * @param {string} sku
   * @returns {Promise<{ sku: string }>}
   */
  @ClearCache({"method":"find"})
  async invalidate(sku) { return { sku }; }
}
