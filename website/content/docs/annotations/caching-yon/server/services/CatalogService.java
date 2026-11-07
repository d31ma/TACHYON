@Service
@CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
class CatalogService {
    Behaviors.Context behaviors;
    int reads = 0;

    @OnStart
    void open() {
        Behaviors.Options options = new Behaviors.Options("public-catalog", new Behaviors.Signal());
        behaviors = Behaviors.create("CatalogSetup", options);
    }

    @Cached
    Object find(String sku) {
        reads++;
        return entry(sku, reads);
    }

    @UpdateCache({"method":"find"})
    Object write(String sku, long reads) { return entry(sku, reads); }

    @ClearCache({"method":"find"})
    Object invalidate(String sku) { return entry(sku, reads); }

    private Object entry(String sku, long reads) { return java.util.Map.of("sku", sku, "reads", reads); }
}
