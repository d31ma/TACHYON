@CacheSettings({"name":"products","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
@Delegate
class CatalogDelegate {
  static _FIRST_WORKING_CALL = 3;
  calls = 0;
  prices = 0;

  @Cached
  async find(id) {
    this.calls++;
    return { id, calls: this.calls };
  }
  @Cached
  async price(@CacheKey sku, note) {
    this.prices++;
    return this.prices;
  }
  @UpdateCache({"method":"find"})
  async replace(id, calls) { return { id, calls }; }
  @ClearCache({"method":"find"})
  async remove(id) { return true; }

  page(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) input) { return input; }

  @Retry({"maxRetries":3,"delayMs":0,"multiplier":1,"maxDelayMs":50,"jitterMs":0})
  @MaxAtOnce({"limit":2,"policy":"block"})
  @AllOrNothing({"adapter":"catalog"})
  async flaky() {
    this.calls++;
    if (this.calls < CatalogDelegate._FIRST_WORKING_CALL) throw Error('transient');
    return this.calls;
  }
}

