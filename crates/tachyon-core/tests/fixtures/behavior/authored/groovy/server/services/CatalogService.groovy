@CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":128})
@Service
class CatalogService {
    private static final String PAGE = "page"
    private static final String UPDATED = "updated"
    Behaviors.Context behaviors
    int calls = 0
    int mutations = 0
    int validCalls = 0
    @Cached
    Object list(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) java.util.Map<String, Object> page) {
        calls++
        return java.util.Map.of("calls", calls, PAGE, page)
    }
    @ClearCache({"method":"list"})
    Object remove(String ignored, java.util.Map<String, Object> page) {
        mutations++
        return true
    }
    @UpdateCache({"method":"list"})
    Object update(String value, java.util.Map<String, Object> page) {
        mutations++
        return java.util.Map.of(UPDATED, value)
    }
    @UpdateCache({"method":"list"})
    Object invalid(java.util.Map<String, Object> page) { return new Object() }
    @Cached
    Object checked(@Check(Input.class) Object input) {
        validCalls++
        return input
    }
}

