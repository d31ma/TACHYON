@CacheSettings({"name":"products","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
@Helper
class CatalogHelper {
  static FIRST_WORKING_CALL = 3;
  calls = 0;

  @Cached
  async find(id) {
    this.calls++;
    return { id, calls: this.calls };
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
    if (this.calls < CatalogHelper.FIRST_WORKING_CALL) throw Error('transient');
    return this.calls;
  }
}

