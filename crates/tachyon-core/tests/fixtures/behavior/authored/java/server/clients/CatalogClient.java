@CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":128})
@HTTPClient({"adapter": "api"})
class CatalogClient {
    private static final String PAGE = "page";
    private static final String UPDATED = "updated";
    private static final String BODY = "body";
    Behaviors.Context behaviors;
    int calls = 0;
    int mutations = 0;
    int validCalls = 0;
    @Cached
    Object list(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) java.util.Map<String, Object> page) {
        calls++;
        return java.util.Map.of("calls", calls, PAGE, page);
    }
    @GetRequest("/products")
    Object remote() { return java.util.Map.of("query", java.util.Map.of(PAGE, 1)); }
    @GetRequest("/products")
    Object search(@RequestParam("page") long page) {}
    @GetRequest("/products/{id}")
    Object find(@PathVariable long id) {}
    @ClearCache({"method":"list"})
    Object remove(String ignored, java.util.Map<String, Object> page) {
        mutations++;
        return true;
    }
    @UpdateCache({"method":"list"})
    Object update(String value, java.util.Map<String, Object> page) {
        mutations++;
        return java.util.Map.of(UPDATED, value);
    }
    @UpdateCache({"method":"list"})
    Object invalid(java.util.Map<String, Object> page) { return new Object(); }
    @UpdateCache({"method":"list"})
    @PostRequest("/products")
    Object remoteUpdate(java.util.Map<String, Object> page) { return java.util.Map.of(BODY, java.util.Map.of(UPDATED, true)); }
    @Cached
    Object checked(@Check(Input.class) Object input) {
        validCalls++;
        return input;
    }
    @PostRequest("/validated")
    Object checkedRemote(@Check(Input.class) Object input) {
        validCalls++;
        return java.util.Map.of(BODY, input);
    }
}

